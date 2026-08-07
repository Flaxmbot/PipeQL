use std::collections::HashMap;
use std::fmt;

use crate::analyzer::Analyzer;
use crate::ast::*;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenError {
    UnsupportedDialect(String),
    InvalidAST(String),
    /// Semantic analysis (column/scope validation) failed.
    Analysis(Vec<crate::analyzer::AnalyzerError>),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::UnsupportedDialect(d) => write!(f, "Unsupported dialect: {d}"),
            CodegenError::InvalidAST(msg) => write!(f, "Invalid AST: {msg}"),
            CodegenError::Analysis(errs) => {
                write!(f, "Analysis failed:")?;
                for e in errs {
                    write!(f, "\n  {}..{}: {}", e.span.start, e.span.end, e.message)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for CodegenError {}

/// A compiler target that renders a PipeQL pipeline into a dialect's SQL plus
/// the ordered list of dynamic parameter values.
pub trait Dialect {
    fn name(&self) -> &str;

    /// The dialect kind, which drives placeholder style and DDL mapping.
    fn kind(&self) -> DialectKind;

    /// Compile a pipeline to SQL with parameter extraction. No schema catalog
    /// is required; column validation is skipped.
    fn compile(&self, pipeline: &Pipeline) -> Result<(String, Vec<String>), CodegenError> {
        self.compile_with_catalog(pipeline, None)
    }

    /// Compile a pipeline, optionally validating columns/scopes against a
    /// schema catalog before generating SQL.
    fn compile_with_catalog(
        &self,
        pipeline: &Pipeline,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<(String, Vec<String>), CodegenError>;

    /// Compile a whole statement (pipeline, insert, or table DDL).
    fn compile_statement(&self, stmt: &Statement) -> Result<(String, Vec<String>), CodegenError> {
        self.compile_statement_with_catalog(stmt, None)
    }

    /// Compile a whole statement, optionally validating against a schema
    /// catalog before generating SQL.
    fn compile_statement_with_catalog(
        &self,
        stmt: &Statement,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<(String, Vec<String>), CodegenError> {
        self.analyze_statement(stmt, catalog)
            .map_err(CodegenError::Analysis)?;
        let mut c = Compiler::new(self.kind());
        let sql = c.compile_statement(stmt)?;
        Ok((sql, c.params))
    }

    /// Run semantic analysis (parameter extraction + optional column
    /// validation) without generating SQL.
    fn analyze(
        &self,
        pipeline: &Pipeline,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<crate::analyzer::Analysis, Vec<crate::analyzer::AnalyzerError>> {
        Analyzer::new(catalog).analyze(pipeline)
    }

    /// Run semantic analysis on a whole statement without generating SQL.
    fn analyze_statement(
        &self,
        stmt: &Statement,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<crate::analyzer::Analysis, Vec<crate::analyzer::AnalyzerError>> {
        Analyzer::new(catalog).analyze_statement(stmt)
    }
}

/// The SQL dialects PipeQL transpiles to. Drives both placeholder style and
/// DDL type/constraint mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialectKind {
    Postgres,
    Sqlite,
    Duckdb,
    Mysql,
}

impl DialectKind {
    /// How a dialect renders dynamic parameters in SQL.
    fn placeholder_style(&self) -> ParamStyle {
        match self {
            DialectKind::Postgres => ParamStyle::Postgres,
            DialectKind::Sqlite | DialectKind::Duckdb | DialectKind::Mysql => ParamStyle::Question,
        }
    }
}

/// How a dialect renders dynamic parameters in SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParamStyle {
    /// PostgreSQL `$1, $2, ...`
    Postgres,
    /// SQLite / DuckDB / MySQL `?`
    Question,
}

/// Shared SQL compiler used by every dialect. Dialects differ only in how
/// parameters are rendered and how DDL is mapped.
struct Compiler {
    kind: DialectKind,
    params: Vec<String>,
    /// Name -> assigned placeholder index. Enables deduplication: the same
    /// parameter bound to `$N` in PostgreSQL may be referenced multiple times.
    param_index: HashMap<String, usize>,
    /// Derived names -> their RHS AST expression, enabling inlining of
    /// downstream references (e.g. `net_pay = salary - tax`).
    derive_exprs: HashMap<String, Expr>,
    /// Aggregate aliases -> their function-call AST, used for HAVING
    /// substitution (PostgreSQL cannot reference SELECT aliases in HAVING).
    aggregate_exprs: HashMap<String, Expr>,
    /// In mutation statements every literal is extracted into the parameter
    /// vector so no raw value is ever inlined into the SQL text.
    strict_literals: bool,
}

impl Compiler {
    fn new(kind: DialectKind) -> Self {
        Self {
            kind,
            params: Vec::new(),
            param_index: HashMap::new(),
            derive_exprs: HashMap::new(),
            aggregate_exprs: HashMap::new(),
            strict_literals: false,
        }
    }

    fn placeholder(&mut self, name: &str) -> String {
        match self.kind.placeholder_style() {
            ParamStyle::Postgres => {
                // Deduplicate: reuse the same $N for repeated parameter names.
                if let Some(&idx) = self.param_index.get(name) {
                    return format!("${}", idx + 1);
                }
                self.params.push(name.to_string());
                let idx = self.params.len();
                self.param_index.insert(name.to_string(), idx - 1);
                format!("${idx}")
            }
            ParamStyle::Question => {
                // Positional `?` placeholders bind once per occurrence, so each
                // use of a parameter supplies its value again.
                self.params.push(name.to_string());
                "?".to_string()
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> String {
        self.compile_expr_inner(expr, true)
    }

    /// Compile an expression without substituting derived names (used for
    /// ORDER BY, where referencing the SELECT alias is valid SQL).
    fn compile_expr_no_sub(&mut self, expr: &Expr) -> String {
        self.compile_expr_inner(expr, false)
    }

    fn compile_expr_inner(&mut self, expr: &Expr, substitute: bool) -> String {
        match expr {
            Expr::Star => "*".to_string(),
            Expr::Ident(id) => {
                if substitute {
                    if let Some(dexpr) = self.derive_exprs.get(&id.name).cloned() {
                        return self.compile_expr_inner(&dexpr, true);
                    }
                }
                id.name.clone()
            }
            Expr::Literal(lit) => match lit {
                Literal::String(v) => self.placeholder(v),
                Literal::Integer(v) if self.strict_literals => self.placeholder(&v.to_string()),
                Literal::Integer(v) => v.to_string(),
                Literal::Float(v) if self.strict_literals => self.placeholder(&v.to_string()),
                Literal::Float(v) => v.to_string(),
                Literal::Bool(v) if self.strict_literals => self.placeholder(&v.to_string()),
                Literal::Bool(v) => v.to_string(),
                Literal::Null => "NULL".to_string(),
            },
            Expr::Parameter(p) => self.placeholder(&p.name),
            Expr::UnaryOp { op, expr } => {
                let inner = self.compile_expr_inner(expr, substitute);
                format!("{op} ({inner})")
            }
            Expr::IsNull { expr, negated } => {
                let inner = self.compile_expr_inner(expr, substitute);
                if *negated {
                    format!("({inner} IS NOT NULL)")
                } else {
                    format!("({inner} IS NULL)")
                }
            }
            Expr::InList {
                expr,
                list,
                negated,
            } => {
                let inner = self.compile_expr_inner(expr, substitute);
                let items = list
                    .iter()
                    .map(|e| self.compile_expr_inner(e, substitute))
                    .collect::<Vec<_>>()
                    .join(", ");
                let keyword = if *negated { "NOT IN" } else { "IN" };
                format!("({inner} {keyword} ({items}))")
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.compile_expr_inner(left, substitute);
                let r = self.compile_expr_inner(right, substitute);
                let op_sql = match op {
                    BinaryOp::Eq => "=",
                    BinaryOp::NotEq => "<>",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::And => "AND",
                    BinaryOp::Or => "OR",
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                };
                format!("({l} {op_sql} {r})")
            }
            Expr::FunctionCall { name, args } => {
                let args_str = args
                    .iter()
                    .map(|a| self.compile_expr_inner(a, substitute))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name.name, args_str)
            }
            Expr::ColumnRef {
                table,
                column,
                json_path,
            } => {
                let mut col = String::new();
                if let Some(t) = table {
                    col.push_str(&t.name);
                    col.push('.');
                }
                col.push_str(&column.name);
                for seg in json_path {
                    col.push_str(&format!("->>'{}'", seg.name));
                }
                col
            }
        }
    }

    /// Replace aggregate aliases with their function-call expressions, so that
    /// HAVING clauses reference the aggregate itself rather than an alias.
    fn with_aggregates(&self, expr: &Expr) -> Expr {
        match expr {
            Expr::Ident(id) => {
                if let Some(agg) = self.aggregate_exprs.get(&id.name) {
                    agg.clone()
                } else {
                    expr.clone()
                }
            }
            Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
                left: Box::new(self.with_aggregates(left)),
                op: *op,
                right: Box::new(self.with_aggregates(right)),
            },
            Expr::UnaryOp { op, expr } => Expr::UnaryOp {
                op: *op,
                expr: Box::new(self.with_aggregates(expr)),
            },
            Expr::IsNull { expr, negated } => Expr::IsNull {
                expr: Box::new(self.with_aggregates(expr)),
                negated: *negated,
            },
            Expr::InList {
                expr,
                list,
                negated,
            } => Expr::InList {
                expr: Box::new(self.with_aggregates(expr)),
                list: list.iter().map(|a| self.with_aggregates(a)).collect(),
                negated: *negated,
            },
            Expr::FunctionCall { name, args } => Expr::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|a| self.with_aggregates(a)).collect(),
            },
            other => other.clone(),
        }
    }

    fn compile_pipeline(&mut self, pipeline: &Pipeline) -> Result<String, CodegenError> {
        if pipeline
            .steps
            .iter()
            .any(|s| matches!(s, PipelineStep::Update { .. } | PipelineStep::Delete { .. }))
        {
            return self.compile_mutation_pipeline(pipeline);
        }

        // Clause buckets collected by walking the PipeQL steps in pipeline order.
        let mut where_clauses: Vec<String> = Vec::new();
        let mut select_columns: Vec<String> = Vec::new();
        let mut has_select = false;
        let mut order_by: Vec<String> = Vec::new();
        let mut limit: Option<i64> = None;
        let mut offset: Option<i64> = None;
        let mut joins: Vec<String> = Vec::new();
        let mut group_by: Vec<String> = Vec::new();
        let mut aggregate_selects: Vec<String> = Vec::new();
        let mut having: Vec<String> = Vec::new();
        let mut grouped = false;
        // Derived columns that are not explicitly selected but should appear
        // when there is no `select` step (SELECT *, <derived> AS ...).
        let mut derived_output: Vec<String> = Vec::new();

        for step in &pipeline.steps {
            match step {
                PipelineStep::Filter { expr, .. } => {
                    if grouped {
                        let with_aggs = self.with_aggregates(expr);
                        having.push(self.compile_expr(&with_aggs));
                    } else {
                        where_clauses.push(self.compile_expr(expr));
                    }
                }
                PipelineStep::Select { columns, .. } => {
                    has_select = true;
                    for item in columns {
                        // A bare identifier referencing a derived column is
                        // inlined and aliased back to its name.
                        if let Expr::Ident(id) = &item.expr {
                            if self.derive_exprs.contains_key(&id.name) {
                                let rendered = self.compile_expr(&item.expr);
                                let alias = item
                                    .alias
                                    .as_ref()
                                    .map(|a| a.name.clone())
                                    .unwrap_or_else(|| id.name.clone());
                                select_columns.push(format!("{rendered} AS {alias}"));
                                continue;
                            }
                        }
                        let rendered = self.compile_expr(&item.expr);
                        if let Some(alias) = &item.alias {
                            select_columns.push(format!("{rendered} AS {}", alias.name));
                        } else {
                            select_columns.push(rendered);
                        }
                    }
                }
                PipelineStep::Derive { assignments, .. } => {
                    for a in assignments {
                        // Compile the RHS (inlining previously derived names),
                        // then register the derived name for later steps.
                        let rendered = self.compile_expr(&a.expr);
                        self.derive_exprs
                            .insert(a.name.name.clone(), a.expr.clone());
                        derived_output.push(format!("{rendered} AS {}", a.name.name));
                    }
                }
                PipelineStep::Join {
                    join_type,
                    table,
                    alias,
                    on,
                    ..
                } => {
                    let join_type_str = match join_type {
                        JoinType::Inner => "INNER JOIN",
                        JoinType::Left => "LEFT JOIN",
                        JoinType::Right => "RIGHT JOIN",
                        JoinType::Full => "FULL OUTER JOIN",
                    };
                    let mut join_sql = format!("{join_type_str} {}", table.name);
                    if let Some(a) = alias {
                        join_sql.push_str(&format!(" AS {}", a.name));
                    }
                    join_sql.push_str(&format!(" ON {}", self.compile_expr(on)));
                    joins.push(join_sql);
                }
                PipelineStep::Group {
                    columns,
                    aggregates,
                    ..
                } => {
                    grouped = true;
                    for col in columns {
                        let col_sql = self.compile_expr(col);
                        group_by.push(col_sql.clone());
                        aggregate_selects.push(col_sql);
                    }
                    for agg in aggregates {
                        let args_str = agg
                            .args
                            .iter()
                            .map(|a| self.compile_expr(a))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let agg_expr = Expr::FunctionCall {
                            name: agg.func.clone(),
                            args: agg.args.clone(),
                        };
                        self.aggregate_exprs.insert(agg.name.name.clone(), agg_expr);
                        aggregate_selects.push(format!(
                            "{}({}) AS {}",
                            agg.func.name.to_uppercase(),
                            args_str,
                            agg.name.name
                        ));
                    }
                }
                PipelineStep::Sort { items, .. } => {
                    for item in items {
                        // ORDER BY may reference SELECT aliases, so we do not
                        // substitute derived names here.
                        let expr = self.compile_expr_no_sub(&item.expr);
                        let dir = match item.direction {
                            SortDirection::Asc => "ASC",
                            SortDirection::Desc => "DESC",
                        };
                        order_by.push(format!("{expr} {dir}"));
                    }
                }
                PipelineStep::Take { count, .. } => {
                    limit = Some(*count);
                }
                PipelineStep::Skip { count, .. } => {
                    offset = Some(*count);
                }
                PipelineStep::Update { .. } | PipelineStep::Delete { .. } => {
                    return Err(CodegenError::InvalidAST(
                        "update/delete in a read pipeline is not supported — \
                         use: from <table> | filter ... | update [...] or | delete"
                            .to_string(),
                    ));
                }
            }
        }

        // Assemble the final SELECT list.
        let final_selects: Vec<String> = if grouped {
            aggregate_selects
        } else if has_select {
            select_columns
        } else if !derived_output.is_empty() {
            let mut v = vec!["*".to_string()];
            v.extend(derived_output);
            v
        } else {
            vec!["*".to_string()]
        };
        let select_list = final_selects.join(", ");

        // Emit SQL in valid execution order.
        let mut sql = String::new();
        sql.push_str(&format!("SELECT {select_list} FROM "));
        sql.push_str(&pipeline.source.name.name);
        if let Some(alias) = &pipeline.source.alias {
            sql.push_str(&format!(" AS {}", alias.name));
        }

        for join in &joins {
            sql.push_str(&format!("\n{join}"));
        }

        if !where_clauses.is_empty() {
            sql.push_str(&format!("\nWHERE {}", where_clauses.join(" AND ")));
        }

        if !group_by.is_empty() {
            sql.push_str(&format!("\nGROUP BY {}", group_by.join(", ")));
        }

        if !having.is_empty() {
            sql.push_str(&format!("\nHAVING {}", having.join(" AND ")));
        }

        if !order_by.is_empty() {
            sql.push_str(&format!("\nORDER BY {}", order_by.join(", ")));
        }

        if let Some(lim) = limit {
            sql.push_str(&format!("\nLIMIT {lim}"));
        }

        if let Some(off) = offset {
            sql.push_str(&format!("\nOFFSET {off}"));
        }

        sql.push(';');
        Ok(sql)
    }

    /// Dispatch a whole statement to the right compiler.
    fn compile_statement(&mut self, stmt: &Statement) -> Result<String, CodegenError> {
        match stmt {
            Statement::Pipeline(p) => self.compile_pipeline(p),
            Statement::Insert(i) => self.compile_insert(i),
            Statement::CreateTable(c) => self.compile_create_table(c),
        }
    }

    /// Compile an `update`/`delete` pipeline. `update` compiles SET
    /// assignments before WHERE filters so the parameter vector lists SET
    /// values first (matching the PRD contract).
    fn compile_mutation_pipeline(&mut self, pipeline: &Pipeline) -> Result<String, CodegenError> {
        let mut filters: Vec<&Expr> = Vec::new();
        let mut assignments: Option<&[Assignment]> = None;
        let mut is_delete = false;
        let mut saw_terminal = false;

        for step in &pipeline.steps {
            match step {
                PipelineStep::Filter { expr, .. } if !saw_terminal => filters.push(expr),
                PipelineStep::Update { assignments: a, .. } if !saw_terminal => {
                    assignments = Some(a);
                    saw_terminal = true;
                }
                PipelineStep::Delete { .. } if !saw_terminal => {
                    is_delete = true;
                    saw_terminal = true;
                }
                PipelineStep::Filter { .. } => {
                    return Err(CodegenError::InvalidAST(
                        "filter after update/delete is not allowed — update/delete must be the terminal step. \
                         Move the filter before the mutation: from t | filter ... | update/delete"
                            .to_string(),
                    ));
                }
                _ => {
                    return Err(CodegenError::InvalidAST(
                        "select, sort, take, skip, join, group, or derive after a mutation is not allowed — \
                         update/delete must be the terminal step. Only filter steps are permitted before a mutation."
                            .to_string(),
                    ));
                }
            }
        }

        if !saw_terminal {
            return Err(CodegenError::InvalidAST(
                "mutation pipeline must contain an update or delete step — \
                 use: from <table> | filter ... | update [...] or | delete"
                    .to_string(),
            ));
        }

        self.strict_literals = true;
        let table = &pipeline.source.name.name;

        let set_sql = if is_delete {
            String::new()
        } else {
            assignments
                .expect("delete has no assignments; update must")
                .iter()
                .map(|a| format!("{} = {}", a.name.name, self.compile_expr(&a.expr)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let where_sql = if filters.is_empty() {
            String::new()
        } else {
            let clauses = filters
                .iter()
                .map(|e| self.compile_expr(e))
                .collect::<Vec<_>>()
                .join(" AND ");
            format!("\nWHERE {clauses}")
        };

        if is_delete {
            Ok(format!("DELETE FROM {table}{where_sql};"))
        } else {
            Ok(format!("UPDATE {table}\nSET {set_sql}{where_sql};"))
        }
    }

    /// Compile an `insert` statement. Every assigned value becomes a
    /// parameter; PostgreSQL additionally returns the inserted row.
    fn compile_insert(&mut self, insert: &InsertStmt) -> Result<String, CodegenError> {
        self.strict_literals = true;
        let cols = insert
            .assignments
            .iter()
            .map(|a| a.name.name.clone())
            .collect::<Vec<_>>();
        let vals = insert
            .assignments
            .iter()
            .map(|a| self.compile_expr(&a.expr))
            .collect::<Vec<_>>();
        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            insert.table.name,
            cols.join(", "),
            vals.join(", ")
        );
        if matches!(self.kind, DialectKind::Postgres) {
            sql.push_str(" RETURNING *");
        }
        sql.push(';');
        Ok(sql)
    }

    /// Compile a `table` DDL statement. DDL carries no parameters.
    fn compile_create_table(&mut self, create: &CreateTableStmt) -> Result<String, CodegenError> {
        let cols = create
            .columns
            .iter()
            .map(|c| self.compile_column_def(c))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "CREATE TABLE IF NOT EXISTS {} (\n  {}\n);",
            create.name.name,
            cols.join(",\n  ")
        ))
    }

    fn compile_column_def(&mut self, col: &ColumnDef) -> Result<String, CodegenError> {
        let mut s = format!("{} {}", col.name.name, self.ddl_type(col.ty));
        for m in &col.modifiers {
            match m {
                ColumnModifier::PrimaryKey => s.push_str(" PRIMARY KEY"),
                ColumnModifier::AutoIncrement => s.push_str(self.ddl_auto_increment()),
                ColumnModifier::NotNull => s.push_str(" NOT NULL"),
                ColumnModifier::Unique => s.push_str(" UNIQUE"),
                ColumnModifier::Default(expr) => {
                    s.push_str(&format!(" DEFAULT {}", self.ddl_default(expr)?));
                }
            }
        }
        Ok(s)
    }

    fn ddl_type(&self, ty: ColumnType) -> &'static str {
        match (self.kind, ty) {
            (DialectKind::Postgres, ColumnType::Integer) => "INTEGER",
            (DialectKind::Postgres, ColumnType::Float) => "DOUBLE PRECISION",
            (DialectKind::Postgres, ColumnType::String) => "TEXT",
            (DialectKind::Postgres, ColumnType::Bool) => "BOOLEAN",
            (DialectKind::Postgres, ColumnType::Timestamp) => "TIMESTAMP",
            (DialectKind::Sqlite, ColumnType::Integer) => "INTEGER",
            (DialectKind::Sqlite, ColumnType::Float) => "REAL",
            (DialectKind::Sqlite, ColumnType::String) => "TEXT",
            (DialectKind::Sqlite, ColumnType::Bool) => "INTEGER",
            (DialectKind::Sqlite, ColumnType::Timestamp) => "DATETIME",
            (DialectKind::Duckdb, ColumnType::Integer) => "INTEGER",
            (DialectKind::Duckdb, ColumnType::Float) => "DOUBLE",
            (DialectKind::Duckdb, ColumnType::String) => "VARCHAR",
            (DialectKind::Duckdb, ColumnType::Bool) => "BOOLEAN",
            (DialectKind::Duckdb, ColumnType::Timestamp) => "TIMESTAMP",
            (DialectKind::Mysql, ColumnType::Integer) => "INT",
            (DialectKind::Mysql, ColumnType::Float) => "DOUBLE",
            (DialectKind::Mysql, ColumnType::String) => "VARCHAR(255)",
            (DialectKind::Mysql, ColumnType::Bool) => "BOOLEAN",
            (DialectKind::Mysql, ColumnType::Timestamp) => "TIMESTAMP",
        }
    }

    fn ddl_auto_increment(&self) -> &'static str {
        match self.kind {
            DialectKind::Postgres => " GENERATED ALWAYS AS IDENTITY",
            DialectKind::Sqlite => " AUTOINCREMENT",
            DialectKind::Duckdb => " GENERATED BY DEFAULT AS IDENTITY",
            DialectKind::Mysql => " AUTO_INCREMENT",
        }
    }

    fn ddl_default(&self, expr: &Expr) -> Result<String, CodegenError> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::String(v) => Ok(format!("'{v}'")),
                Literal::Integer(v) => Ok(v.to_string()),
                Literal::Float(v) => Ok(v.to_string()),
                Literal::Bool(v) => Ok(v.to_string()),
                Literal::Null => Ok("NULL".to_string()),
            },
            Expr::Ident(id) => {
                if id.name.eq_ignore_ascii_case("current_timestamp") {
                    Ok("CURRENT_TIMESTAMP".to_string())
                } else {
                    Ok(id.name.clone())
                }
            }
            Expr::FunctionCall { name, args } => {
                let args_str = args
                    .iter()
                    .map(|a| self.ddl_default(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}({args_str})", name.name.to_uppercase()))
            }
            _ => Err(CodegenError::InvalidAST(
                "unsupported DEFAULT expression in table DDL".to_string(),
            )),
        }
    }
}

pub struct PostgresDialect;

impl Dialect for PostgresDialect {
    fn name(&self) -> &str {
        "postgres"
    }

    fn kind(&self) -> DialectKind {
        DialectKind::Postgres
    }

    fn compile_with_catalog(
        &self,
        pipeline: &Pipeline,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<(String, Vec<String>), CodegenError> {
        Analyzer::new(catalog)
            .analyze(pipeline)
            .map_err(CodegenError::Analysis)?;
        let mut c = Compiler::new(self.kind());
        let sql = c.compile_pipeline(pipeline)?;
        Ok((sql, c.params))
    }
}

pub struct SQLiteDialect;

impl Dialect for SQLiteDialect {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn kind(&self) -> DialectKind {
        DialectKind::Sqlite
    }

    fn compile_with_catalog(
        &self,
        pipeline: &Pipeline,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<(String, Vec<String>), CodegenError> {
        Analyzer::new(catalog)
            .analyze(pipeline)
            .map_err(CodegenError::Analysis)?;
        let mut c = Compiler::new(self.kind());
        let sql = c.compile_pipeline(pipeline)?;
        Ok((sql, c.params))
    }
}

pub struct DuckDBDialect;

impl Dialect for DuckDBDialect {
    fn name(&self) -> &str {
        "duckdb"
    }

    fn kind(&self) -> DialectKind {
        DialectKind::Duckdb
    }

    fn compile_with_catalog(
        &self,
        pipeline: &Pipeline,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<(String, Vec<String>), CodegenError> {
        Analyzer::new(catalog)
            .analyze(pipeline)
            .map_err(CodegenError::Analysis)?;
        let mut c = Compiler::new(self.kind());
        let sql = c.compile_pipeline(pipeline)?;
        Ok((sql, c.params))
    }
}

pub struct MySQLDialect;

impl Dialect for MySQLDialect {
    fn name(&self) -> &str {
        "mysql"
    }

    fn kind(&self) -> DialectKind {
        DialectKind::Mysql
    }

    fn compile_with_catalog(
        &self,
        pipeline: &Pipeline,
        catalog: Option<&crate::analyzer::Catalog>,
    ) -> Result<(String, Vec<String>), CodegenError> {
        Analyzer::new(catalog)
            .analyze(pipeline)
            .map_err(CodegenError::Analysis)?;
        let mut c = Compiler::new(self.kind());
        let sql = c.compile_pipeline(pipeline)?;
        Ok((sql, c.params))
    }
}

pub fn get_dialect(name: &str) -> Result<Box<dyn Dialect>, CodegenError> {
    match name {
        "postgres" | "postgresql" => Ok(Box::new(PostgresDialect)),
        "sqlite" => Ok(Box::new(SQLiteDialect)),
        "duckdb" => Ok(Box::new(DuckDBDialect)),
        "mysql" => Ok(Box::new(MySQLDialect)),
        _ => Err(CodegenError::UnsupportedDialect(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_postgres_basic() {
        let source = "from users | filter age > 18 | select [id, name]";
        let mut parser = Parser::new(source).unwrap();
        let pipeline = parser.parse_pipeline().unwrap();

        let dialect = PostgresDialect;
        let (sql, _params) = dialect.compile(&pipeline).unwrap();

        assert!(sql.contains("SELECT id, name FROM users"));
        assert!(sql.contains("WHERE (age > 18)"));
    }

    #[test]
    fn test_postgres_params() {
        let source = "from users | filter name == $name and age >= $min_age";
        let mut parser = Parser::new(source).unwrap();
        let pipeline = parser.parse_pipeline().unwrap();

        let dialect = PostgresDialect;
        let (sql, params) = dialect.compile(&pipeline).unwrap();

        assert!(sql.contains("$1"));
        assert!(sql.contains("$2"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "name");
        assert_eq!(params[1], "min_age");
    }
}
