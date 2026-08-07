use std::fmt;

/// A span representing a range in the source text (byte offset start..end).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Identifier with a span.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

/// Literal values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Integer(v) => write!(f, "{v}"),
            Literal::Float(v) => write!(f, "{v}"),
            Literal::String(v) => write!(f, "'{v}'"),
            Literal::Bool(v) => write!(f, "{v}"),
            Literal::Null => write!(f, "NULL"),
        }
    }
}

/// Binary operators.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Eq => write!(f, "="),
            BinaryOp::NotEq => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::LtEq => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::GtEq => write!(f, ">="),
            BinaryOp::And => write!(f, "AND"),
            BinaryOp::Or => write!(f, "OR"),
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
        }
    }
}

/// A dynamic parameter reference ($param or ${param}).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub span: Span,
}

/// A unary operator.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Not => write!(f, "NOT"),
        }
    }
}

/// A comment in the source, preserved for lossless round-tripping.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub text: String,
    pub span: Span,
}

/// Expression AST node.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Star,
    Ident(Ident),
    Literal(Literal),
    Parameter(Parameter),
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    IsNull {
        expr: Box<Expr>,
        negated: bool,
    },
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    InSubquery {
        expr: Box<Expr>,
        subquery: Box<Pipeline>,
        negated: bool,
    },
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    FunctionCall {
        name: Ident,
        args: Vec<Expr>,
    },
    ColumnRef {
        table: Option<Ident>,
        column: Ident,
        json_path: Vec<Ident>,
    },
}

/// Sort direction.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// A sort item (column + direction).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SortItem {
    pub expr: Expr,
    pub direction: SortDirection,
}

/// Join type.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// A column assignment in derive: `name = expression`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub name: Ident,
    pub expr: Expr,
}

/// A column in a select list with optional alias: `expr as alias`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<Ident>,
}

/// An aggregate in group: `name = func(args)`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregate {
    pub name: Ident,
    pub func: Ident,
    pub args: Vec<Expr>,
}

/// A column type in `table` DDL declarations.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    String,
    Bool,
    Timestamp,
}

/// A column constraint/modifier in `table` DDL declarations.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnModifier {
    PrimaryKey,
    AutoIncrement,
    NotNull,
    Unique,
    Default(Expr),
}

/// A single column definition: `name type [modifiers...]`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: Ident,
    pub ty: ColumnType,
    pub modifiers: Vec<ColumnModifier>,
}

/// The statement forms PipeQL supports.
///
/// `Pipeline` is serialized untagged so the JSON shape for v1.0 read queries
/// stays byte-for-byte identical (`{steps, comments, span}`), while mutations
/// serialize as their own structs.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged)
)]
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Pipeline(Pipeline),
    Insert(InsertStmt),
    Upsert(UpsertStmt),
    CreateTable(CreateTableStmt),
    Union(UnionStmt),
}

/// An `insert` statement: `into <table> | insert [assignments]`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStmt {
    pub table: Ident,
    pub assignments: Vec<Assignment>,
    /// All comments encountered in the source, in source order, so the AST is
    /// lossless and can be used for tooling (LSP, formatters, tree-sitter).
    pub comments: Vec<Comment>,
    pub span: Span,
}

/// An `upsert` statement: `into <table> | upsert [...] | conflict [...] | do update [...]`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct UpsertStmt {
    pub table: Ident,
    pub assignments: Vec<Assignment>,
    pub conflict_columns: Vec<Ident>,
    pub do_update: Vec<Assignment>,
    pub comments: Vec<Comment>,
    pub span: Span,
}

/// A `union` statement: `<left> | union [all] <right>`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct UnionStmt {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
    pub all: bool,
    pub span: Span,
}

/// A `table` DDL statement: `table <name> [column_defs]`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTableStmt {
    pub name: Ident,
    pub columns: Vec<ColumnDef>,
    /// All comments encountered in the source, in source order, so the AST is
    /// lossless and can be used for tooling (LSP, formatters, tree-sitter).
    pub comments: Vec<Comment>,
    pub span: Span,
}

/// Pipeline step variants.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStep {
    Filter {
        expr: Expr,
        span: Span,
    },
    Select {
        columns: Vec<SelectItem>,
        span: Span,
    },
    Derive {
        assignments: Vec<Assignment>,
        span: Span,
    },
    Join {
        join_type: JoinType,
        table: Ident,
        alias: Option<Ident>,
        on: Expr,
        span: Span,
    },
    Group {
        columns: Vec<Expr>,
        aggregates: Vec<Aggregate>,
        span: Span,
    },
    Sort {
        items: Vec<SortItem>,
        span: Span,
    },
    Take {
        count: i64,
        span: Span,
    },
    Skip {
        count: i64,
        span: Span,
    },
    Update {
        assignments: Vec<Assignment>,
        span: Span,
    },
    Delete {
        span: Span,
    },
}

/// The source table reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct TableSource {
    pub name: Ident,
    pub alias: Option<Ident>,
}

/// The top-level Pipeline AST.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub source: TableSource,
    pub steps: Vec<PipelineStep>,
    /// All comments encountered in the source, in source order, so the AST is
    /// lossless and can be used for tooling (LSP, formatters, tree-sitter).
    pub comments: Vec<Comment>,
}
