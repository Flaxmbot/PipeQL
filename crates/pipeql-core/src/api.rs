//! High-level, ergonomic API for compiling PipeQL sources.
//!
//! This is the facade used by the CLI, the polyglot bindings (WASM, Python,
//! C-FFI), and application code. It ties together lexer, parser, analyzer, and
//! codegen into a single call.

use std::fmt;

use crate::analyzer::{Analysis, AnalyzerError, Catalog};
use crate::ast::Statement;
use crate::codegen::{get_dialect, CodegenError};
use crate::parser::{ParseError, Parser};

/// The kind of statement a compiled query represents. Drivers use this to
/// choose the right execution path (return rows vs. execute + metadata) without
/// inspecting the SQL text.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StatementType {
    /// A read pipeline: `from <table> | ...` without update/delete.
    Select,
    /// `into <table> | insert [...]`.
    Insert,
    /// `from <table> | [filters] | update [...]`.
    Update,
    /// `from <table> | [filters] | delete`.
    Delete,
    /// `table <name> [column defs]` DDL.
    CreateTable,
}

impl StatementType {
    /// Classify a parsed statement AST.
    pub fn from_statement(stmt: &crate::ast::Statement) -> StatementType {
        match stmt {
            crate::ast::Statement::Insert(_) => StatementType::Insert,
            crate::ast::Statement::CreateTable(_) => StatementType::CreateTable,
            crate::ast::Statement::Pipeline(p) => {
                if p.steps
                    .iter()
                    .any(|s| matches!(s, crate::ast::PipelineStep::Update { .. }))
                {
                    StatementType::Update
                } else if p
                    .steps
                    .iter()
                    .any(|s| matches!(s, crate::ast::PipelineStep::Delete { .. }))
                {
                    StatementType::Delete
                } else {
                    StatementType::Select
                }
            }
        }
    }

    /// The stable snake_case string form (`"insert"`, `"create_table"`, ...).
    pub fn as_str(&self) -> &'static str {
        match self {
            StatementType::Select => "select",
            StatementType::Insert => "insert",
            StatementType::Update => "update",
            StatementType::Delete => "delete",
            StatementType::CreateTable => "create_table",
        }
    }

    /// True for statements that write (insert/update/delete).
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            StatementType::Insert | StatementType::Update | StatementType::Delete
        )
    }
}

impl fmt::Display for StatementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A fully compiled query: dialect SQL plus the ordered parameter vector.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledQuery {
    pub sql: String,
    pub params: Vec<String>,
    /// The statement kind, so callers can dispatch `.all()` vs `.run()` without
    /// parsing the SQL prefix.
    pub statement_type: StatementType,
    /// Convenience flag: true for insert/update/delete.
    pub is_mutation: bool,
    pub analysis: Analysis,
}

/// Unified error type covering parsing, analysis, and codegen.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum PipeQLError {
    /// Lexer or parser failures (lexer errors are surfaced as parse errors
    /// with spans).
    Parse(Vec<ParseError>),
    /// Semantic analysis failures (column/scope validation).
    Analysis(Vec<AnalyzerError>),
    /// Codegen failures (unsupported dialect, invalid AST).
    Codegen(CodegenError),
}

impl fmt::Display for PipeQLError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipeQLError::Parse(errs) => {
                for e in errs {
                    write!(f, "{e}")?;
                }
                Ok(())
            }
            PipeQLError::Analysis(errs) => {
                for e in errs {
                    write!(
                        f,
                        "Analysis error at {}..{}: {}",
                        e.span.start, e.span.end, e.message
                    )?;
                }
                Ok(())
            }
            PipeQLError::Codegen(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PipeQLError {}

/// Compile a PipeQL source string for the given dialect.
pub fn compile(source: &str, dialect: &str) -> Result<CompiledQuery, PipeQLError> {
    compile_with_catalog(source, dialect, None)
}

/// Compile a PipeQL source string, optionally validating columns against a
/// schema catalog.
pub fn compile_with_catalog(
    source: &str,
    dialect: &str,
    catalog: Option<&Catalog>,
) -> Result<CompiledQuery, PipeQLError> {
    let stmt = parse_statement(source)?;
    let d = get_dialect(dialect).map_err(PipeQLError::Codegen)?;
    let statement_type = StatementType::from_statement(&stmt);
    let (sql, params, analysis) = match &stmt {
        Statement::Pipeline(pipeline) => {
            let analysis = d
                .analyze(pipeline, catalog)
                .map_err(PipeQLError::Analysis)?;
            let (sql, params) = d
                .compile_with_catalog(pipeline, catalog)
                .map_err(PipeQLError::Codegen)?;
            (sql, params, analysis)
        }
        Statement::Insert(_) | Statement::CreateTable(_) => {
            let analysis = d
                .analyze_statement(&stmt, catalog)
                .map_err(PipeQLError::Analysis)?;
            let (sql, params) = d
                .compile_statement_with_catalog(&stmt, catalog)
                .map_err(PipeQLError::Codegen)?;
            (sql, params, analysis)
        }
    };
    Ok(CompiledQuery {
        sql,
        params,
        statement_type,
        is_mutation: statement_type.is_mutation(),
        analysis,
    })
}

/// Parse a PipeQL source into a lossless AST (preserving comments and spans).
/// Returns read pipelines only; use [`parse_statement`] for mutations/DDL.
pub fn parse(source: &str) -> Result<crate::ast::Pipeline, PipeQLError> {
    let mut parser = Parser::new(source).map_err(PipeQLError::Parse)?;
    parser.parse_pipeline().map_err(PipeQLError::Parse)
}

/// Parse a PipeQL source into a lossless statement AST (read pipeline, insert,
/// or table DDL), preserving comments and spans.
pub fn parse_statement(source: &str) -> Result<crate::ast::Statement, PipeQLError> {
    let mut parser = Parser::new(source).map_err(PipeQLError::Parse)?;
    parser.parse_statement().map_err(PipeQLError::Parse)
}

/// The list of supported target dialect names.
pub fn supported_dialects() -> Vec<&'static str> {
    vec!["postgres", "sqlite", "duckdb", "mysql"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_facade() {
        let result = compile("from users | filter age > $min | select [id]", "postgres").unwrap();
        assert!(result.sql.contains("SELECT id FROM users"));
        assert_eq!(result.params, vec!["min"]);
        assert_eq!(result.analysis.param_names(), vec!["min"]);
    }

    #[test]
    fn test_compile_error_surfaces() {
        let err = compile("from users | explode [id]", "postgres").unwrap_err();
        assert!(matches!(err, PipeQLError::Parse(_)));
    }

    #[test]
    fn test_unsupported_dialect() {
        let err = compile("from t", "oracle").unwrap_err();
        assert!(matches!(
            err,
            PipeQLError::Codegen(CodegenError::UnsupportedDialect(_))
        ));
    }

    #[test]
    fn test_statement_type_select() {
        let result = compile("from users | filter age > $min | select [id]", "postgres").unwrap();
        assert_eq!(result.statement_type, StatementType::Select);
        assert!(!result.is_mutation);
        assert_eq!(result.statement_type.as_str(), "select");
    }

    #[test]
    fn test_statement_type_insert() {
        let result = compile("into notes | insert [title = $t, is_pinned = 0]", "sqlite").unwrap();
        assert_eq!(result.statement_type, StatementType::Insert);
        assert!(result.is_mutation);
    }

    #[test]
    fn test_statement_type_update() {
        let result = compile(
            "from notes | filter id == $id | update [is_pinned = 1]",
            "sqlite",
        )
        .unwrap();
        assert_eq!(result.statement_type, StatementType::Update);
        assert!(result.is_mutation);
    }

    #[test]
    fn test_statement_type_delete() {
        let result = compile("from notes | filter id == $id | delete", "sqlite").unwrap();
        assert_eq!(result.statement_type, StatementType::Delete);
        assert!(result.is_mutation);
    }

    #[test]
    fn test_statement_type_create_table() {
        let result = compile("table notes [id int primary auto]", "sqlite").unwrap();
        assert_eq!(result.statement_type, StatementType::CreateTable);
        assert!(!result.is_mutation);
        assert_eq!(result.statement_type.as_str(), "create_table");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_statement_type_serializes_snake_case() {
        let json = serde_json::to_string(&StatementType::CreateTable).unwrap();
        assert_eq!(json, "\"create_table\"");
    }
}
