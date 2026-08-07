# PipeQL Language Specification v2.0.0

PipeQL is a **pipelined query language** that compiles to multiple SQL dialects.
This document is the authoritative reference for syntax, semantics, and the
compilation contract.

## 1. Overview

A PipeQL program is a **statement**. Five statement kinds are supported:

| Statement | Purpose                                   | Keyword      |
| --------- | ----------------------------------------- | ------------ |
| Pipeline  | Read data (`SELECT ...`)                  | `from`       |
| Insert    | Create records (`INSERT`)                 | `into`       |
| Upsert    | Insert or update (`INSERT ... ON CONFLICT`)| `into`       |
| Union     | Combine results (`UNION`)                 | `union`      |
| Table     | Declare schema (`CREATE TABLE`)           | `table`      |

A **pipeline** is a single source table followed by zero or more transformation
steps. Steps are separated by `|` or a newline (consecutive separators
collapse).

```
from <table> [alias]
| step1
| step2
...
```

An **insert statement** targets a table with `into` and assigns column values:

```
into <table>
| insert [ <col> = <value>, ... ]
```

A **table statement** declares columns and constraints:

```
table <name> [ <col> <type> [modifiers...], ... ]
```

Compilation produces:

1. **SQL** in the target dialect, with every non-identifier value replaced by a
   positional placeholder (`$1`, `?`, ...).
2. **An ordered parameter vector** (e.g. `["min", "threshold", ...]`),
   deduplicated by name and ordered by first occurrence.
3. **A semantic analysis** (param map, inferred value types, catalog matches).

No untrusted value ever appears inside generated SQL text. This is the core
injection-safety guarantee: **parameter extraction is structural, not
convention-based.** It holds for reads *and* writes — every literal assigned in
`insert` or `update` becomes a bind parameter.

## 2. Lexical structure

### 2.1 Comments

- `-- line comment` — to end of line.
- `/* block comment */` — may span lines.

Comments are preserved in the lossless AST and surfaced by the parser.

### 2.2 Identifiers

```
identifier ::= [a-zA-Z_][a-zA-Z0-9_]*
```

Keywords are reserved and cannot be used as bare identifiers.

### 2.3 Keywords

| Group        | Keywords                                                                  |
| ------------ | ------------------------------------------------------------------------- |
| Read         | `from`, `filter`, `select`, `derive`, `join`, `group`, `sort`, `take`, `skip`, `as`, `on` |
| Mutation     | `into`, `insert`, `update`, `delete`                                      |
| Schema       | `table`, `primary`, `auto`, `not null`, `unique`, `default`               |
| Types        | `int`, `integer`, `float`, `real`, `string`, `text`, `bool`, `boolean`, `timestamp`, `datetime` |
| Boolean      | `and`, `or`, `not`, `in`, `is`, `null`, `true`, `false`                   |

### 2.4 Literals

| Kind   | Example           |
| ------ | ----------------- |
| Integer | `42`, `0`        |
| Float   | `3.14`, `0.5`    |
| String  | `'active'`       |
| Bool    | `true`, `false`  |
| Null    | `null`           |

String literals use single quotes; doubled quotes (`''`) escape a quote.
**String literals are always compiled to bind parameters**, never inlined.

### 2.5 Parameters

```
parameter ::= '$' identifier | '${' identifier '}'
```

`$min`, `${min}`, `$2` (Postgres-style) are parameter references. Parameters
are **deduplicated by name** in the output vector; each occurrence is tracked.

## 3. Statements

### 3.1 Pipeline (read)

```
from <table> [alias]
| step1
| step2
```

Compiles to `SELECT`. See §4 for steps and §6.1 for clause order.

### 3.2 Insert

```
into <table>
| insert [ <col> = <value>, ... ]
```

`<value>` is any expression (§5). Every value is a bind parameter **except**
`NULL` and the bare identifier `current_timestamp`, which are inlined as SQL
`NULL` and `CURRENT_TIMESTAMP` respectively. Compiles to:

```
INSERT INTO <table> (cols) VALUES (params)
```

On Postgres a `RETURNING *` clause is appended so generated primary keys are
returned (PRD §4.1). Question-style dialects omit it.

### 3.3 Upsert

```
into <table>
| upsert [ <col> = <value>, ... ]
| conflict [ <col>, ... ]
| do update [ <col> = <value>, ... ]
```

The `upsert` statement performs an insert-or-update (UPSERT). The `conflict`
clause specifies which columns trigger the conflict resolution. The `do update`
clause specifies which columns to update on conflict. Every value is a bind
parameter. Compiles to:

| Dialect  | SQL |
| -------- | --- |
| Postgres | `INSERT INTO <table> (cols) VALUES (params) ON CONFLICT (conflict_cols) DO UPDATE SET ... RETURNING *` |
| SQLite   | `INSERT INTO <table> (cols) VALUES (params) ON CONFLICT (conflict_cols) DO UPDATE SET ...` |
| DuckDB   | `INSERT INTO <table> (cols) VALUES (params) ON CONFLICT (conflict_cols) DO UPDATE SET ...` |
| MySQL    | `INSERT INTO <table> (cols) VALUES (params) ON DUPLICATE KEY UPDATE ...` |

MySQL uses `ON DUPLICATE KEY UPDATE` instead of `ON CONFLICT ... DO UPDATE SET`.
The `conflict` columns are semantically required but not rendered in MySQL SQL.

### 3.4 Update

```
from <table>
| filter <condition>
| update [ <col> = <value>, ... ]
```

`update` is a **terminal** pipeline step. Only `filter` steps may precede it
inside a mutation pipeline; any other step is a compile error. Compiles to:

```
UPDATE <table>
SET <col> = <param>, ...
WHERE <filter...>;
```

**Parameter order:** `SET` assignment values come first, in order, then `WHERE`
filter values. `NULL` and `current_timestamp` inline as in §3.2.

### 3.5 Delete

```
from <table>
| filter <condition>
| delete
```

`delete` is a **terminal** step; only `filter` steps may precede it. Compiles
to:

```
DELETE FROM <table>
WHERE <filter...>;
```

### 3.6 Union

```
<statement>
| union [all]
<statement>
```

The `union` keyword chains two statements to combine their result sets. When
`all` follows `union`, duplicate rows are preserved (`UNION ALL`). Without `all`,
duplicates are removed (`UNION`). Both sides must be valid PipeQL statements
(pipelines, inserts, upserts, or unions). Compiles to:

```
<left SQL>
UNION [ALL]
<right SQL>;
```

**Parameter order:** Parameters from the left statement come first, followed by
parameters from the right statement.

### 3.7 Table DDL

```
table <name> [
  <col> <type> [modifiers...],
  ...
]
```

Column types and modifiers:

| Piece       | Values                                                             |
| ----------- | ------------------------------------------------------------------ |
| Type        | `int`/`integer`, `float`/`real`, `string`/`text`, `bool`/`boolean`, `timestamp`/`datetime` |
| Modifiers   | `primary`, `auto`, `not null`, `unique`, `default <expression>`    |

Compiles to `CREATE TABLE IF NOT EXISTS <name> (...);`. Column defaults are
**schema metadata, evaluated by the server** — so their literals are inlined
(quoted for strings) and `current_timestamp` inlines as `CURRENT_TIMESTAMP`; no
DDL default is parameterized.

## 4. Pipeline steps

| Step     | Syntax                                                  |
| -------- | ------------------------------------------------------- |
| filter   | `filter <expression>`                                   |
| select   | `select [<expr> [as name], ...]`                        |
| derive   | `derive [name = <expr>, ...]`                           |
| join     | `[left|right|full|inner] join <table> [alias] on <expr>`|
| group    | `group [<expr>, ...] (name = func(<expr>), ...)`        |
| sort     | `sort [<expr> [asc|desc], ...]`                         |
| take     | `take <number>`                                         |
| skip     | `skip <number>`                                         |
| update   | `update [name = <expr>, ...]` (terminal)                |
| delete   | `delete` (terminal)                                     |

- `join` defaults to `inner`.
- `select` / `derive` / `update` lists may span lines inside `[...]`.
- `group` groups by the bracketed expressions and computes the parenthesized
  aggregates.
- `update` and `delete` are only valid as the final step of a mutation
  pipeline; a mutation pipeline's preceding steps must all be `filter`.

## 5. Expressions

```
expression ::= or_expression

or_expression    ::= and_expression (('or') and_expression)*
and_expression   ::= not_expression (('and') not_expression)*
not_expression   ::= 'not' not_expression | comparison
comparison       ::= additive (('=='|'='|'!='|'<'|'<='|'>'|'>='|'<>') additive)?
                   | additive ['not'] 'in' '[' expression (',' expression)* ']'
                   | additive ['not'] 'in' '(' pipeline ')'
                   | additive 'is' ['not'] 'null'
additive         ::= multiplicative (('+'|'-') multiplicative)*
multiplicative   ::= unary (('*'|'/') unary)*
unary            ::= '-' primary | primary
primary          ::= literal | parameter | column_ref | function_call
                   | 'star' | '(' expression ')'
column_ref       ::= identifier ('.' identifier)*
function_call    ::= identifier '(' expression (',' expression)* ')'
```

- `=` and `==` are equivalent equality operators (both compile to SQL `=`).
- `and` binds tighter than `or`; `not` binds tighter than both.
- `in` accepts a bracket list of expressions or a subquery (`in (from ...)`).
- `*` (star) is a valid primary for `select [*]` and `count(*)`.
- Column references may be dotted (`users.id`, `u.profile.name`); JSON-ish
  paths are codegen-mapped per dialect (e.g. Postgres `->>`).

## 6. Compilation contract

### 6.1 Read pipeline clause order

1. `SELECT <select list>` (or `*`)
2. `FROM <source>` (aliased)
3. `JOIN ... ON ...`
4. `WHERE <filter>`
5. `GROUP BY <group keys>`
6. `HAVING <aggregate filters>`
7. `ORDER BY <sort items>`
8. `LIMIT <take>` / `OFFSET <skip>`

### 6.2 Mutation shape

| Statement | SQL shape                                                  |
| --------- | ---------------------------------------------------------- |
| insert    | `INSERT INTO t (c1, c2) VALUES (p1, p2)` + `RETURNING *` (postgres) |
| upsert   | `INSERT INTO t (...) VALUES (...) ON CONFLICT (...) DO UPDATE SET ...` (or `ON DUPLICATE KEY UPDATE` for MySQL) |
| update    | `UPDATE t SET c1 = p1, ... WHERE <filters>` (SET before WHERE) |
| delete    | `DELETE FROM t WHERE <filters>`                            |
| table     | `CREATE TABLE IF NOT EXISTS t (col type constraints, ...)` |

### 6.3 Placeholder conventions

| Dialect   | Placeholder     |
| --------- | --------------- |
| postgres  | `$1`, `$2`, ... |
| sqlite    | `?` (per occurrence) |
| duckdb    | `?` (per occurrence) |
| mysql     | `?` (per occurrence) |

Postgres parameters are deduplicated by name **and by identical literal
value** — a value repeated across a query (e.g. `0` in two filters) binds to the
same `$N`. Question-style dialects emit one placeholder per occurrence.

### 6.4 Expression mapping

- PipeQL `==`/`=` → SQL `=`
- PipeQL `!=`/`<>` → SQL `<>` (postgres/sqlite/duckdb) or `!=` (mysql)
- PipeQL `and`/`or`/`not` → SQL `AND`/`OR`/`NOT`
- PipeQL `is [not] null` → SQL `IS [NOT] NULL`
- PipeQL `in [...]` → SQL `IN (...)`
- PipeQL `in (from ...)` → SQL `IN (SELECT ...)`
- JSON dotted path → dialect accessor (Postgres `->>`, SQLite `->>`/`json_extract`,
  DuckDB `->>`/`get_json_string`, MySQL `JSON_UNQUOTE(JSON_EXTRACT(...))`)
- Aggregate functions (`sum`, `count`, `min`, `max`, `avg`) → SQL same-name.

### 6.5 Parameter extraction (universal)

Every literal and `$parameter` in the query becomes a bind parameter — in
`SELECT` filters **and** in `insert`/`update` assignments. Exceptions that are
inlined into SQL text (never parameterized):

| Construct             | Inlined as            |
| --------------------- | --------------------- |
| `NULL`                | SQL `NULL`            |
| bare `current_timestamp` | SQL `CURRENT_TIMESTAMP` |
| DDL column defaults   | quoted/typed literal (schema metadata) |

In update/delete, `SET` values precede `WHERE` values in the parameter vector,
matching placeholder order in the generated SQL.

### 6.6 DDL type mapping

| PipeQL type        | postgres           | sqlite   | duckdb       | mysql         |
| ------------------ | ------------------ | -------- | ------------ | ------------- |
| `int`/`integer`    | `INTEGER`          | `INTEGER`| `INTEGER`    | `INT`         |
| `float`/`real`     | `DOUBLE PRECISION` | `REAL`   | `DOUBLE`     | `DOUBLE`      |
| `string`/`text`    | `TEXT`             | `TEXT`   | `VARCHAR`    | `VARCHAR(255)`|
| `bool`/`boolean`   | `BOOLEAN`          | `INTEGER`| `BOOLEAN`    | `BOOLEAN`     |
| `timestamp`/`datetime` | `TIMESTAMP`    | `DATETIME`| `TIMESTAMP` | `TIMESTAMP`   |

Auto-increment (`auto`) maps per dialect:

| Dialect | Rendering                                |
| ------- | ---------------------------------------- |
| postgres| `GENERATED ALWAYS AS IDENTITY`           |
| sqlite  | `AUTOINCREMENT`                          |
| duckdb  | `GENERATED BY DEFAULT AS IDENTITY`       |
| mysql   | `AUTO_INCREMENT`                         |

## 7. Semantic analysis

The analyzer can validate a read pipeline against an optional **schema
catalog**:

- Column references are resolved to tables in scope.
- Unknown columns/tables raise analysis errors (with spans and hints).
- Value types are inferred from literals and catalog metadata
  (`integer`, `float`, `string`, `bool`, `null`, `any`).

When no catalog is supplied, scope validation is skipped; parameter extraction
and type inference still run. Mutations extract parameters structurally without
catalog validation; mutation pipelines are validated for step legality
(filter-only prefix, terminal update/delete).

### 7.1 `$data` object expansion (driver-level extension)

The `$data` token is **not part of the compiler** — the official drivers
(`@pipeql/js/driver`, `pipeql_python.driver`) rewrite it into explicit column
assignments before compilation:

- Insert: the whole bound object becomes the data object:
  `into notes | insert $data` with `{ title, category }` →
  `insert [title = $data0, category = $data1]`, values bound positionally.
- Update: the object is read from `params.data`, and SET parameters always
  precede WHERE parameters:
  `from notes | filter id == $id | update $data` with
  `{ id, data: { title } }` → `update [title = $data0]` + `WHERE id = $id`.
- `$data` must map to a non-empty object, else the driver raises a clear error.

The compiler sees only the expanded (fully explicit) source, so SQL output is
identical to hand-written PipeQL with zero ABI/compiler changes.

## 8. Errors

All errors carry a **byte span** and an optional **hint**. Three classes:

| Class    | Meaning                              |
| -------- | ------------------------------------ |
| Parse    | Lexing/parsing failed                |
| Analysis | Semantic check failed (catalog/scope)|
| Codegen  | Unsupported dialect or invalid AST   |

Errors are rendered consistently across the CLI, WASM, Python, and C APIs.

## 9. Formal grammar (EBNF)

```
statement    := NEWLINE* (pipeline | insert_stmt | upsert_stmt | union_stmt | table_stmt) EOF
pipeline     := source (SEP step)*
source       := 'from' IDENT (IDENT)?
SEP          := ('|' | NEWLINE)+
step         := filter_step | select_step | derive_step | join_step
              | group_step | sort_step | take_step | skip_step
              | update_step | delete_step
filter_step  := 'filter' expression
select_step  := 'select' '[' (select_item (',' select_item)*)? ']'
select_item  := expression ('as' IDENT)?
derive_step  := 'derive' '[' (assignment (',' assignment)*)? ']'
join_step    := ('left'|'right'|'full'|'inner')? 'join' IDENT (IDENT)? 'on' expression
group_step   := 'group' '[' (expression (',' expression)*)? ']'
               ('(' (aggregate (',' aggregate)*)? ')')?
aggregate    := IDENT '=' IDENT '(' (expression (',' expression)*)? ')'
sort_step    := 'sort' '[' (sort_item (',' sort_item)*)? ']'
sort_item    := expression ('asc' | 'desc')?
take_step    := 'take' NUMBER
skip_step    := 'skip' NUMBER
update_step  := 'update' '[' (assignment (',' assignment)*)? ']'
delete_step  := 'delete'
insert_stmt  := 'into' IDENT SEP 'insert' '[' (assignment (',' assignment)*)? ']'
upsert_stmt  := 'into' IDENT SEP 'upsert' '[' (assignment (',' assignment)*)? ']'
               SEP 'conflict' '[' (IDENT (',' IDENT)*)? ']'
               SEP 'do' 'update' '[' (assignment (',' assignment)*)? ']'
union_stmt   := statement SEP ('union' ('all')?) SEP statement
table_stmt   := 'table' IDENT '[' (column_def (',' column_def)*)? ']'
column_def   := IDENT column_type column_modifier*
column_type  := 'int' | 'integer' | 'float' | 'real' | 'string' | 'text'
              | 'bool' | 'boolean' | 'timestamp' | 'datetime'
column_modifier := 'primary' | 'auto' | 'not' 'null' | 'unique'
                 | 'default' expression
assignment   := IDENT '=' expression
```

Semantic constraints not captured in the grammar:

- In a mutation pipeline the `update`/`delete` step is terminal and every
  preceding step must be `filter`.
- `update`/`delete` may not be followed by any step.
- `union` chains two statements; both must be valid PipeQL statements.

## 10. Non-functional requirements (measured)

| Target (PRD)              | Measured              | Status |
| ------------------------- | --------------------- | ------ |
| Read transpile < 0.5ms (50-line) | ~19µs avg / ~31µs worst | ✓ |
| Mutation transpile < 25µs/query | ~8.3µs avg / ~9.8µs worst | ✓ |
| WASM bundle < 600KB gzip  | ~115KB gzip           | ✓      |
| 100% parameter extraction | string + `$param` + `$n`, mutation values | ✓ |
| `#![deny(unsafe_code)]`   | enforced in core      | ✓      |
