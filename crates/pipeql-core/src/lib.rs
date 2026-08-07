#![deny(unsafe_code)]

pub mod analyzer;
pub mod api;
pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;

pub use analyzer::{
    Analysis, Analyzer, AnalyzerError, Catalog, ColumnMeta, ParamMeta, TableMeta, ValueType,
};
pub use api::{
    compile, compile_with_catalog, parse, parse_statement, supported_dialects, CompiledQuery,
    PipeQLError, StatementType,
};
pub use ast::*;
pub use codegen::{
    get_dialect, CodegenError, Dialect, DialectKind, DuckDBDialect, MySQLDialect, PostgresDialect,
    SQLiteDialect,
};
pub use lexer::{Lexer, LexerError, Token, TokenKind};
pub use parser::{ParseError, Parser};
