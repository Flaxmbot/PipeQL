<p align="center">
  <img src="https://raw.githubusercontent.com/Flaxmbot/PipeQL/master/logo.png" alt="PipeQL Logo" width="220" />
</p>

# tree-sitter-pipeql

<p align="center">
  <a href="https://github.com/Flaxmbot/PipeQL/actions"><img src="https://img.shields.io/github/actions/workflow/status/Flaxmbot/PipeQL/ci.yml?style=flat-square&logo=github&label=CI" alt="CI" /></a>
  <a href="https://www.npmjs.com/package/tree-sitter-pipeql"><img src="https://img.shields.io/npm/v/tree-sitter-pipeql?style=flat-square&logo=npm&logoColor=white&color=38bdf8" alt="npm" /></a>
  <a href="https://github.com/Flaxmbot/PipeQL/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-c084fc?style=flat-square" alt="License" /></a>
</p>

A [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) grammar for
**PipeQL** — the pipelined, injection-safe, polyglot query language.

It powers syntax highlighting, code folding, and editor tooling (VS Code
extension, LSP) for `.pql` / `.pipeql` files.

## Features

- Full statement grammar: read pipelines (`from <table>` + `filter` / `select`
  / `derive` / `join` / `group` / `sort` / `take` / `skip` steps), inserts
  (`into <table> | insert [...]`), and schema DDL (`table <name> [columns]`).
- Mutation steps: `update [...]` and `delete` (terminal pipeline steps).
- Steps separated by `|` or newlines (consecutive separators collapse, matching
  the Rust core parser).
- Complete expression grammar with `and` / `or` / `not`, comparisons
  (`==`, `=`, `!=`, `<`, `<=`, `>`, `>=`, `<>`), `in`, `is [not] null`,
  arithmetic, function calls, and `$param` / `${param}` parameters.
- DDL column types (`int`, `float`, `string`, `bool`, `timestamp`, ...) and
  modifiers (`primary`, `auto`, `not null`, `unique`, `default <expr>`).
- Lossless comments (`--` line and `/* */` block) preserved as nodes.
- Highlighting, locals, and tags queries.

## Usage

### Install / build

```bash
npm install          # builds the native Node binding (requires a C++ toolchain)
npm run generate     # regenerate src/ from grammar.js
npm test             # run the corpus test suite
```

To build a standalone parser library for other languages:

```bash
npx tree-sitter build --wasm        # WASM (for browser / VS Code web)
cargo build --release               # Rust (bindings/rust via Cargo.toml)
make                                 # C (produces libtree-sitter-pipeql)
```

### Highlighting

```bash
npx tree-sitter highlight --scope source.pipeql path/to/query.pql
```

### In VS Code

The `pipeql.pipeql-language` extension in `extensions/vscode-pipeql` consumes
this grammar for syntax highlighting and code folding.

## Grammar overview

```
statement          := pipeline | insert_statement | table_statement
pipeline           := source (step_with_sep)* trailing?
source             := 'from' identifier (identifier)?
step_with_sep      := ('|' | '\n')+ step
step               := filter | select | derive | join | group | sort | take | skip | update | delete
filter_step        := 'filter' expression
select_step        := 'select' '[' select_item (',' select_item)* ']'
derive_step        := 'derive' '[' assignment (',' assignment)* ']'
update_step        := 'update' '[' assignment (',' assignment)* ']'
delete_step        := 'delete'
join_step          := ('left' | 'right' | 'full' | 'inner')? 'join' identifier (identifier)? 'on' expression
group_step         := 'group' '[' expression (',' expression)* ']' ('(' aggregate (',' aggregate)* ')')?
sort_step          := 'sort' '[' sort_item (',' sort_item)* ']'
take_step          := 'take' number
skip_step          := 'skip' number
insert_statement   := 'into' identifier (step_with_sep)* 'insert' '[' assignment (',' assignment)* ']'
table_statement    := 'table' identifier '[' column_def (',' column_def)* ']'
column_def         := identifier column_type (column_modifier)*
column_type        := 'int' | 'integer' | 'float' | 'real' | 'string' | 'text' | 'bool' | 'boolean' | 'timestamp' | 'datetime'
column_modifier    := 'primary' | 'auto' | 'unique' | 'not null' | 'default' expression
```

## Layout

```
grammar.js                  grammar definition (source of truth)
src/                        generated C parser + token scanner
queries/                    highlights.scm, locals.scm, tags.scm, injections.scm
bindings/                   node, c, go, python, rust bindings
test/corpus/                parser test cases
```

## Development

After changing `grammar.js`:

```bash
npm run generate
npx tree-sitter test        # corpus tests
```

Add new cases to `test/corpus/pipeline.txt`, run `npx tree-sitter test`,
and update expected trees with `npx tree-sitter test -u` when the change is
intentional.

## License

MIT
