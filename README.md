<p align="center">
  <img src="https://raw.githubusercontent.com/Flaxmbot/PipeQL/master/logo.png" alt="PipeQL Logo" width="250" />
</p>

<h3 align="center">Pipelined, Injection-Safe, Polyglot Query & Mutation Language</h3>

<p align="center">
  <a href="https://github.com/Flaxmbot/PipeQL/actions"><img src="https://img.shields.io/github/actions/workflow/status/Flaxmbot/PipeQL/ci.yml?branch=main&style=flat-square&logo=github&label=CI" alt="CI Status" /></a>
  <a href="https://npmjs.com/package/@flaxmbot/pipeql"><img src="https://img.shields.io/npm/v/@flaxmbot/pipeql?style=flat-square&logo=npm&logoColor=white&color=38bdf8" alt="npm" /></a>
  <a href="https://pypi.org/project/pipeql"><img src="https://img.shields.io/pypi/v/pipeql?style=flat-square&logo=pypi&logoColor=white&color=818cf8" alt="PyPI" /></a>
  <a href="https://github.com/Flaxmbot/PipeQL/releases/tag/v1.1.0"><img src="https://img.shields.io/badge/release-v1.1.0-10b981?style=flat-square&logo=github" alt="Release" /></a>
  <a href="https://github.com/Flaxmbot/PipeQL/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-c084fc?style=flat-square" alt="License" /></a>
</p>

<p align="center">
  <a href="https://github.com/Flaxmbot/PipeQL">
    <img src="https://img.shields.io/badge/View%20on%20GitHub-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub" />
  </a>
</p>

---

Write queries once in a clean, left-to-right UNIX pipeline syntax (`|`). PipeQL compiles directly to target-native SQL for **PostgreSQL**, **SQLite**, **DuckDB**, and **MySQL** in **~19µs**, with 100% structural parameter extraction.

```pipeql
from orders
| join customers on orders.customer_id == customers.id
| filter orders.status == 'active' and orders.total >= $min
| group [region] (total = sum(orders.total), cnt = count(*))
| filter total > $threshold
| select [region, total, cnt]
| sort [total desc]
| take 10
```

Compiles to PostgreSQL:

```sql
SELECT region, SUM(orders.total) AS total, COUNT(*) AS cnt FROM orders
INNER JOIN customers ON (orders.customer_id = customers.id)
WHERE ((orders.status = $1) AND (orders.total >= $2))
GROUP BY region
HAVING (sum(orders.total) > $3)
ORDER BY total DESC
LIMIT 10;
```

**Parameters:** `["active", "min", "threshold"]`. Every string literal and `$param` reference is extracted at the AST level into positionally bound parameter arrays.

---

## Language Ecosystem & Availability Matrix

| Language / SDK | Package / Binding | Status | Dialects Supported |
| :--- | :--- | :--- | :--- |
| **Rust** | `pipeql-core` (Native Crate) | ✅ **Supported** | Postgres, SQLite, DuckDB, MySQL |
| **JavaScript / TypeScript** | `@flaxmbot/pipeql` (WASM) | ✅ **Supported** | Postgres, SQLite, DuckDB, MySQL |
| **Python** | `pipeql` (PyO3 ABI3) | ✅ **Supported** | Postgres, SQLite, DuckDB, MySQL |
| **C / C++** | `libpipeql` (CFFI Header) | ✅ **Supported** | Postgres, SQLite, DuckDB, MySQL |
| **Go** | `pipeql/go` (CGO Bridge) | ✅ **Supported** | Postgres, SQLite, DuckDB, MySQL |

---

## Installation & Setup Guide

### 1. Pre-built CLI & Native Shared Libraries

Download pre-compiled release binaries and shared CFFI libraries directly from [GitHub Releases (v1.1.0)](https://github.com/Flaxmbot/PipeQL/releases/tag/v1.1.0):

| Platform | CLI Executable | Shared Library (CFFI / Go) |
| :--- | :--- | :--- |
| **Windows (x64)** | `pipeql-windows-x86_64.exe` | `pipeql_cffi.dll` |
| **Linux (x64)** | `pipeql-linux-x86_64` | `libpipeql_cffi.so` |
| **macOS (x64)** | `pipeql-macos-x86_64` | `libpipeql_cffi.dylib` |

#### Using CLI directly:
```bash
# Compile PipeQL query to PostgreSQL SQL
./pipeql-linux-x86_64 compile "from users | filter age >= $min | select [id, name]" --dialect postgres

# Compile PipeQL query to SQLite SQL
pipeql-windows-x86_64.exe compile "from notes | filter id == $id" --dialect sqlite
```

---

### 2. Rust Core Crate & CLI

Build from source or add `pipeql-core` to your `Cargo.toml`:

```toml
[dependencies]
pipeql-core = { git = "https://github.com/Flaxmbot/PipeQL.git" }
```

Build & install CLI locally:
```bash
cargo install --path crates/pipeql-cli
pipeql compile "from users | take 10" --dialect postgres
```

---

### 3. JavaScript / TypeScript (Node.js & WebAssembly)

Install from npm or from the GitHub release package:

```bash
# Install via npm
npm install @flaxmbot/pipeql

# Or install directly from GitHub release tarball
npm install https://github.com/Flaxmbot/PipeQL/releases/download/v1.1.0/flaxmbot-pipeql-1.1.0.tgz
```

#### Usage:
```javascript
import { compile } from '@flaxmbot/pipeql';

const { sql, params } = compile(
  "from notes | filter category == $cat | sort [updated_at desc]",
  "sqlite"
);

console.log(sql);    // SELECT * FROM notes WHERE (category = ?) ORDER BY updated_at DESC;
console.log(params); // ["cat"]
```

---

### 4. Python (`pipeql`)

Install from PyPI or install the release wheel:

```bash
# Install via pip
pip install pipeql

# Or build locally using maturin
pip install maturin
maturin develop -m crates/pipeql-python/Cargo.toml
```

#### Usage:
```python
import pipeql_python as pipeql

res = pipeql.compile("into users | insert [name = $name, email = $email]", "postgres")
print(res["sql"])    # INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *;
print(res["params"]) # ["name", "email"]
```

---

### 5. C / C++ (`libpipeql`)

Include the CFFI header `crates/pipeql-cffi/include/libpipeql.h` and link against `libpipeql_cffi`:

#### Build CFFI library locally:
```bash
cargo build --release -p pipeql-cffi
# Linux: target/release/libpipeql_cffi.so
# macOS: target/release/libpipeql_cffi.dylib
# Windows: target/release/pipeql_cffi.dll
```

#### C Example (`demo.c`):
```c
#include <stdio.h>
#include "libpipeql.h"

int main() {
    PipeqlError err = {0};
    PipeqlResult* res = pipeql_compile("from users | filter id == $id", "postgres", &err);
    if (!res) {
        fprintf(stderr, "Error: %s\n", err.message);
        return 1;
    }
    printf("Generated SQL: %s\n", res->sql);
    pipeql_result_free(res);
    return 0;
}
```
Compile & link:
```bash
gcc demo.c -I./crates/pipeql-cffi/include -L./target/release -lpipeql_cffi -o demo
./demo
```

---

### 6. Go (`pipeql/go`)

Import the Go binding and link `libpipeql_cffi`:

```bash
go get github.com/Flaxmbot/PipeQL/go
```

#### Go Example:
```go
package main

import (
    "fmt"
    "log"
    "github.com/Flaxmbot/PipeQL/go"
)

func main() {
    res, err := pipeql.Compile("from users | filter age >= $min | select [id, name]", "postgres")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("SQL:", res.SQL)
    fmt.Println("Params:", res.Params) // ["min"]
}
```

---

## Features

- **4 Dialects in One Pass**: PostgreSQL (`$1`), SQLite (`?`), DuckDB (`?`), MySQL (`?`).
- **100% Parameter Extraction**: Strings (`'active'`) and explicit parameters (`$min`, `${min}`) are automatically extracted into typed bind parameters.
- **Lossless AST**: Spans and comments survive parsing for IDE language servers (`pipeql-lsp`) and formatters.
- **Sub-Millisecond Speed**: Measured average compilation latency of **~19µs** per query.
- **Zero Unsafe Core**: Enforces `#![deny(unsafe_code)]` across the entire compiler engine.
- **Upsert**: Insert-or-update with `ON CONFLICT ... DO UPDATE SET` (Postgres/SQLite/DuckDB) or `ON DUPLICATE KEY UPDATE` (MySQL).
- **Subqueries**: Nested pipelines via `in (from ...)` for correlated and uncorrelated subqueries.
- **Union / Union All**: Combine result sets from multiple statements with `union` or `union all`.
- **Live Playground**: Interactive browser-based playground with WASM compilation for all 4 dialects.

---

## AI & LLM Integration (System Prompt)

PipeQL is designed for first-class AI code generation. The repository includes an optimized **LLM System Prompt** ([ai/system_prompt.md](file:///d:/PipeQL/ai/system_prompt.md)) that instructs models (OpenAI GPT-4, Claude, Gemini, LangChain, etc.) on how to write valid, injection-safe PipeQL code.

### Accessing the System Prompt:

- **Python SDK**:
  ```python
  import pipeql_python

  # Access the pre-loaded LLM System Prompt string
  system_prompt = pipeql_python.SYSTEM_PROMPT
  ```

- **JavaScript / Node.js**:
  Included in the `@flaxmbot/pipeql` npm package at `@flaxmbot/pipeql/ai/system_prompt.md`.

- **GitHub Release / Direct Link**:
  Download [pipeql-ai-system-prompt.md](https://github.com/Flaxmbot/PipeQL/releases/tag/v1.1.0) or fetch directly via raw URL:
  `https://raw.githubusercontent.com/Flaxmbot/PipeQL/master/ai/system_prompt.md`

---

## Architecture

The compilation pipeline has 3 stages:

1. **Source Lexing** — Hand-written lexer tokenizes inputs preserving character positions (for LSP/IDE support).
2. **Parsing & AST** — Pratt parser translates tokens into a lossless abstract syntax tree.
3. **Parameter Isolation + SQL Codegen** — Parser walks the AST, extracts all constants into bind parameters, and generates dialect-specific SQL.

All language bindings (JS, Python, C, Go) are thin wrappers that call the Rust core through `pipeql-core`'s `api.rs` facade.

---

## Project Structure

```
PipeQL/
├── crates/
│   ├── pipeql-core/        # Core compiler (lexer, parser, AST, codegen)
│   ├── pipeql-cli/         # CLI tool
│   ├── pipeql-cffi/        # C ABI shared library (libpipeql_cffi)
│   ├── pipeql-wasm/        # WebAssembly target
│   ├── pipeql-python/      # Python bindings (PyO3)
│   └── pipeql-lsp/         # Language server protocol
├── js/                     # JavaScript/TypeScript SDK (@pipeql/js)
├── python/                 # Python package
├── go/                     # Go binding (CGO)
├── docs/                   # Specification and documentation
├── docs-web/               # Interactive documentation website
├── examples/               # Sample .pql query files
├── extensions/             # VS Code extension
├── tree-sitter-pipeql/     # Tree-sitter grammar
└── Notes/                  # Example CRUD application
```

---

## License

MIT
