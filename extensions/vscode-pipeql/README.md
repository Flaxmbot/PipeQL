<p align="center">
  <img src="https://raw.githubusercontent.com/Flaxmbot/PipeQL/master/logo.png" alt="PipeQL Logo" width="220" />
</p>

<h1 align="center">PipeQL for Visual Studio Code</h1>

<p align="center">
  <a href="https://marketplace.visualstudio.com/items?itemName=pipeql.vscode-pipeql"><img src="https://img.shields.io/visual-studio-marketplace/v/pipeql.vscode-pipeql?style=flat-square&logo=visualstudiocode&color=0078d4" alt="VS Marketplace" /></a>
  <a href="https://github.com/Flaxmbot/PipeQL/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-c084fc?style=flat-square" alt="License" /></a>
</p>

Language support for **PipeQL** — the pipelined, injection-safe, polyglot query
language.

## Features

- Syntax highlighting for `.pql` / `.pipeql` files (TextMate grammar mirroring
  the `tree-sitter-pipeql` scopes).
- Language server integration (diagnostics, keyword completion, hover) via the
  `pipeql-lsp` binary.
- Snippets for pipelines, filters, joins, group-by, derives, sort, pagination,
  and `insert` / `update` / `delete` / `upsert` / `union` / `table` statements,
  including the `update-all` / `delete-all` full-table escape hatches.
- `PipeQL: Compile Query to SQL` command — compiles the active query to a target
  dialect and opens the SQL in a side-by-side editor.

## Requirements

- **LSP**: build `pipeql-lsp` with `cargo build -p pipeql-lsp` and either place
  `pipeql-lsp` (or `pipeql-lsp.exe`) in `PATH`, in `bin/` inside the extension,
  or point `pipeql.lsp.path` at it. Highlighting and snippets work without it.
- **Compile command**: `pipeql` CLI from `crates/pipeql-cli` (or set
  `pipeql.cliPath`).

## Configuration

| Setting                 | Default    | Description                          |
| ----------------------- | ---------- | ------------------------------------ |
| `pipeql.lsp.enabled`    | `true`     | Toggle the language server           |
| `pipeql.lsp.path`       | `""`       | Path to the `pipeql-lsp` executable  |
| `pipeql.cliPath`        | `""`       | Path to the `pipeql` CLI             |
| `pipeql.defaultDialect` | `postgres` | SQL dialect for diagnostics/preview  |

## Development

```bash
npm install          # install deps (vscode-languageclient, typescript)
npm run compile      # tsc -> out/
```

Open the extension folder in VS Code and press F5 to launch an Extension
Development Host.
