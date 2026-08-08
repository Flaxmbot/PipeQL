//! PipeQL Language Server: diagnostics, completion, and hover via the LSP
//! protocol (tower-lsp). Consumed by editors such as VS Code (see
//! `extensions/vscode-pipeql`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pipeql_core::ast::{PipelineStep, Statement};
use pipeql_core::codegen::{get_dialect, CodegenError};
use pipeql_core::{api, PipeQLError};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Render a codegen-level failure as a diagnostic. Codegen errors carry no
/// source span, so the diagnostic covers the whole document — the error is
/// statement-level, and pointing at a hardcoded (0,0) would be misleading.
fn codegen_diag(e: CodegenError, text: &str) -> Diagnostic {
    let end = offset_to_position(text.len(), text);
    Diagnostic {
        range: Range::new(Position::default(), end),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("pipeql-codegen".into())),
        source: Some("pipeql".into()),
        message: e.to_string(),
        ..Default::default()
    }
}

/// Convert a byte offset to a 0-based (line, character) position.
///
/// LSP positions use UTF-16 code units for `character`, so multi-byte chars
/// (emoji, CJK, accents) must count as 2 — counting chars would make
/// diagnostics after the first non-ASCII char land on the wrong column.
fn offset_to_position(offset: usize, text: &str) -> Position {
    let mut line = 0usize;
    let mut col = 0usize;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16();
        }
    }
    Position::new(line as u32, col as u32)
}

fn to_range(span: (usize, usize), text: &str) -> Range {
    Range::new(
        offset_to_position(span.0, text),
        offset_to_position(span.1, text),
    )
}

/// Collect spans of explicit full-table mutations (`update all` / `delete all`)
/// so the LSP can flag them as hint-level lint diagnostics. The bool marks
/// delete (`true`) vs. update (`false`).
fn collect_full_table_mutations(stmt: &Statement, out: &mut Vec<(usize, usize, bool)>) {
    match stmt {
        Statement::Pipeline(p) => {
            for step in &p.steps {
                match step {
                    PipelineStep::Update { all: true, span, .. } => {
                        out.push((span.start, span.end, false));
                    }
                    PipelineStep::Delete { all: true, span, .. } => {
                        out.push((span.start, span.end, true));
                    }
                    _ => {}
                }
            }
        }
        Statement::Union(u) => {
            collect_full_table_mutations(&u.left, out);
            collect_full_table_mutations(&u.right, out);
        }
        _ => {}
    }
}/// Build LSP diagnostics for a document. The source is parsed exactly once;
/// the same AST drives both compile-error reporting and the lint pass.
fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    let stmt = api::parse_statement(text);

    let mut diags = match &stmt {
        Err(PipeQLError::Parse(errs)) => errs
            .iter()
            .map(|e| Diagnostic {
                range: to_range((e.span.start, e.span.end), text),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("pipeql-parse".into())),
                source: Some("pipeql".into()),
                message: e.suggestion.as_ref().map_or_else(
                    || e.message.clone(),
                    |s| format!("{} (hint: {s})", e.message),
                ),
                ..Default::default()
            })
            .collect(),
        Ok(stmt) => match get_dialect("postgres") {
            // "postgres" is always registered, but keep the arm for totality.
            Err(e) => vec![codegen_diag(e, text)],
            Ok(d) => match d.compile_statement_with_catalog(stmt, None) {
                Ok(_) => Vec::new(),
                Err(CodegenError::Analysis(errs)) => errs
                    .iter()
                    .map(|e| Diagnostic {
                        range: to_range((e.span.start, e.span.end), text),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("pipeql-analysis".into())),
                        source: Some("pipeql".into()),
                        message: e.suggestion.as_ref().map_or_else(
                            || e.message.clone(),
                            |s| format!("{} (hint: {s})", e.message),
                        ),
                        ..Default::default()
                    })
                    .collect(),
                Err(e) => vec![codegen_diag(e, text)],
            },
        },
        Err(_) => Vec::new(),
    };

    // Hint-level lint: flag explicit full-table mutations so reviewers see
    // them in diagnostics even though compilation succeeds.
    if let Ok(stmt) = &stmt {
        let mut mutations = Vec::new();
        collect_full_table_mutations(stmt, &mut mutations);
        for (start, end, is_delete) in mutations {
            diags.push(Diagnostic {
                range: to_range((start, end), text),
                severity: Some(DiagnosticSeverity::HINT),
                code: Some(NumberOrString::String("pipeql-full-table".into())),
                source: Some("pipeql".into()),
                message: if is_delete {
                    "`delete all` runs an unfiltered DELETE on the whole table \
                     — confirm this is intentional"
                        .into()
                } else {
                    "`update all` runs an unfiltered UPDATE on every row \
                     — confirm this is intentional"
                        .into()
                },
                ..Default::default()
            });
        }
    }
    diags
}

struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncKind::FULL.into()),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![" ".into(), "$".into(), ".".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "pipeql-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let url = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        if let Ok(mut docs) = self.documents.lock() {
            docs.insert(url.clone(), text);
        }
        let diagnostics = diagnostics_for(&params.text_document.text);
        self.client
            .publish_diagnostics(url, diagnostics, Some(params.text_document.version))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.last() {
            let url = params.text_document.uri.clone();
            if let Ok(mut docs) = self.documents.lock() {
                docs.insert(url.clone(), change.text.clone());
            }
            let diagnostics = diagnostics_for(&change.text);
            self.client
                .publish_diagnostics(url, diagnostics, Some(params.text_document.version))
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let text = self
            .documents
            .lock()
            .ok()
            .and_then(|docs| docs.get(&params.text_document.uri).cloned());
        if let Some(text) = text {
            let diagnostics = diagnostics_for(&text);
            self.client
                .publish_diagnostics(params.text_document.uri, diagnostics, None)
                .await;
        }
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let keywords = [
            "from", "into", "table", "insert", "upsert", "update", "delete", "union", "filter", "select", "derive",
            "join", "group", "sort", "take", "skip",
        ];
        let words = [
            "left", "right", "full", "inner", "as", "on", "and", "or", "not", "in", "is", "null",
            "true", "false", "asc", "desc", "conflict", "do", "all",
        ];
        let types = ["int", "float", "string", "bool", "timestamp", "datetime"];
        let modifiers = ["primary", "auto", "unique", "default"];
        let descriptions = [
            ("from", "Define the source table"),
            ("into", "Target a table for an insert/upsert"),
            ("table", "Declare a table schema"),
            ("insert", "Insert assigned values into the target table"),
            ("upsert", "Insert or update on conflict"),
            ("update", "Update rows matched by preceding filters"),
            ("delete", "Delete rows matched by preceding filters"),
            ("union", "Combine result sets of two statements"),
        ];
        let items: Vec<CompletionItem> =
            keywords
                .iter()
                .map(|k| {
                    CompletionItem::new_simple(
                        (*k).to_string(),
                        descriptions
                            .iter()
                            .find(|(kw, _)| kw == k)
                            .map(|(_, d)| (*d).to_string())
                            .unwrap_or_else(|| "PipeQL step".into()),
                    )
                })
                .chain(words.iter().map(|w| {
                    CompletionItem::new_simple(
                        (*w).to_string(),
                        if w == &"not" {
                            "Negate an expression".into()
                        } else {
                            "PipeQL keyword".into()
                        },
                    )
                }))
                .chain(types.iter().map(|t| {
                    CompletionItem::new_simple((*t).to_string(), "DDL column type".into())
                }))
                .chain(modifiers.iter().map(|m| {
                    CompletionItem::new_simple((*m).to_string(), "DDL column modifier".into())
                }))
                .collect();
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**PipeQL**\n\nPipelined, injection-safe polyglot query language. Compiles to Postgres, SQLite, DuckDB, and MySQL.".into(),
            }),
            range: None,
        }))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(Mutex::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
