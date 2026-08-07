//! PipeQL Language Server: diagnostics, completion, and hover via the LSP
//! protocol (tower-lsp). Consumed by editors such as VS Code (see
//! `extensions/vscode-pipeql`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pipeql_core::{api, PipeQLError};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Convert a byte offset to a 0-based (line, character) position.
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
            col += 1;
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

/// Build LSP diagnostics for a document by running lexer/parser/analyzer.
fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    match api::compile(text, "postgres") {
        Ok(_) => Vec::new(),
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
        Err(PipeQLError::Analysis(errs)) => errs
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
        Err(PipeQLError::Codegen(e)) => vec![Diagnostic {
            range: Range::new(Position::default(), Position::default()),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("pipeql-codegen".into())),
            source: Some("pipeql".into()),
            message: e.to_string(),
            ..Default::default()
        }],
    }
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
