<p align="center">
  <img src="https://raw.githubusercontent.com/Flaxmbot/PipeQL/master/logo.png" alt="PipeQL Logo" width="200" />
</p>

<h1 align="center">PipeQL</h1>
<h4 align="center">Pipelined · Injection-Safe · Polyglot Query Language</h4>

<p align="center">
  <a href="https://github.com/Flaxmbot/PipeQL/actions"><img src="https://img.shields.io/github/actions/workflow/status/Flaxmbot/PipeQL/ci.yml?style=flat-square&logo=github&label=CI" alt="CI" /></a>
  <a href="https://crates.io/crates/pipeql-core"><img src="https://img.shields.io/crates/v/pipeql-core?style=flat-square&logo=rust&logoColor=white&color=e6522c" alt="crates.io" /></a>
  <a href="https://npmjs.com/package/@flaxmbot/pipeql"><img src="https://img.shields.io/npm/v/@flaxmbot/pipeql?style=flat-square&logo=npm&logoColor=white&color=38bdf8" alt="npm" /></a>
  <a href="https://pypi.org/project/pipeql-python"><img src="https://img.shields.io/pypi/v/pipeql-python?style=flat-square&logo=pypi&logoColor=white&color=818cf8" alt="PyPI" /></a>
  <a href="https://github.com/Flaxmbot/PipeQL/releases/latest"><img src="https://img.shields.io/github/v/release/Flaxmbot/PipeQL?style=flat-square&logo=github&color=10b981" alt="Release" /></a>
  <a href="https://github.com/Flaxmbot/PipeQL/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-c084fc?style=flat-square" alt="License" /></a>
</p>

<p align="center">
  <a href="https://pipeql.vercel.app">Docs & Live Playground</a> ·
  <a href="https://github.com/Flaxmbot/PipeQL/releases/latest">Download</a> ·
  <a href="#install">Install</a> ·
  <a href="#syntax-reference">Syntax</a>
</p>

---

Write queries in a clean, left-to-right **UNIX pipeline** syntax. PipeQL compiles them to target-native **parameterized SQL** for PostgreSQL, SQLite, DuckDB, and MySQL — in **~19µs**, with **zero** chance of SQL injection.

```
from orders
| join customers on orders.customer_id == customers.id
| filter orders.status == 'active' and orders.total >= $min
| group [region] (total = sum(orders.total), cnt = count(*))
| filter total > $threshold
| select [region, total, cnt]
| sort [total desc]
| take 10
```

**→ PostgreSQL output:**

```sql
SELECT region, SUM(orders.total) AS total, COUNT(*) AS cnt
FROM orders
INNER JOIN customers ON (orders.customer_id = customers.id)
WHERE ((orders.status = $1) AND (orders.total >= $2))
GROUP BY region HAVING (SUM(orders.total) > $3)
ORDER BY total DESC LIMIT 10;
-- Parameters: [$1='active', $2=min, $3=threshold]
```

Every string literal and `$param` is extracted into a positional bind array at the AST level. **No string concatenation. No escaping. No injection.**

---

## Why PipeQL?

| Problem | PipeQL Solution |
|:---|:---|
| SQL injection | 100% AST-level parameter isolation — impossible to inject |
| Dialect lock-in | Write once, compile to Postgres / SQLite / DuckDB / MySQL |
| Right-to-left SQL | Clean left-to-right pipeline: `from → filter → select → sort` |
| Slow template engines | Native Rust compiler, ~19µs per query |
| Language silos | One compiler, 6 SDKs: Rust, JS/TS, Python, C, C++, Go |

---

<h2 id="install">Install</h2>

### Rust (CLI + Library)

```bash
cargo install pipeql-cli          # CLI tool
```
```toml
# Cargo.toml
[dependencies]
pipeql-core = "1.1"
```

### JavaScript / TypeScript

```bash
npm install @flaxmbot/pipeql
```

### Python

```bash
pip install pipeql-python
```

### Go

> **Note:** The Go binding uses CGO and requires the `libpipeql_cffi` shared library to be built and available on the system.

```bash
# 1. Build the shared library
git clone https://github.com/Flaxmbot/PipeQL.git && cd PipeQL
cargo build --release -p pipeql-cffi

# 2. Install the shared library
# Linux:
sudo cp target/release/libpipeql_cffi.so /usr/local/lib/ && sudo ldconfig
# macOS:
sudo cp target/release/libpipeql_cffi.dylib /usr/local/lib/
# Windows: Add target/release/ to PATH or copy pipeql_cffi.dll alongside your binary

# 3. Add the Go module
go get github.com/Flaxmbot/PipeQL/go@latest
```

### C / C++

```bash
cargo build --release -p pipeql-cffi
# Header: crates/pipeql-cffi/include/libpipeql.h
# Library: target/release/libpipeql_cffi.{so,dylib,dll}
```

> **C Package Managers:** For distributing C/C++ libraries, [**vcpkg**](https://vcpkg.io) (Microsoft, cross-platform, 2400+ packages) and [**Conan**](https://conan.io) (decentralized, supports custom remotes) are the two leading choices. vcpkg is recommended for its CMake integration and cross-platform support.

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/Flaxmbot/PipeQL/releases/latest):

| Platform | CLI | Shared Library |
|:---|:---|:---|
| Linux x64 | `pipeql-linux-x86_64` | `libpipeql_cffi.so` |
| macOS x64 | `pipeql-macos-x86_64` | `libpipeql_cffi.dylib` |
| Windows x64 | `pipeql-windows-x86_64.exe` | `pipeql_cffi.dll` |

---

<h2 id="syntax-reference">Syntax Reference</h2>

### Pipeline Stages

Every PipeQL query starts with a source table and chains stages with `|`:

```
from <table> [as <alias>]
| <stage>
| <stage>
| ...
```

| Stage | Syntax | SQL Equivalent |
|:---|:---|:---|
| **from** | `from users` | `FROM users` |
| **from (alias)** | `from users as u` | `FROM users u` |
| **filter** | `filter age >= 18 and active == true` | `WHERE age >= 18 AND active = TRUE` |
| **select** | `select [id, name, email]` | `SELECT id, name, email` |
| **select (alias)** | `select [full_name as name]` | `SELECT full_name AS name` |
| **select (star)** | `select [*]` | `SELECT *` |
| **derive** | `derive [total = price * qty]` | `SELECT *, (price * qty) AS total` |
| **join** | `join orders on users.id == orders.user_id` | `INNER JOIN orders ON ...` |
| **left join** | `left join orders on users.id == orders.uid` | `LEFT JOIN orders ON ...` |
| **right join** | `right join roles on users.role_id == roles.id` | `RIGHT JOIN roles ON ...` |
| **full join** | `full join archive on a.id == archive.id` | `FULL JOIN archive ON ...` |
| **group** | `group [region] (total = sum(amount))` | `GROUP BY region` |
| **sort** | `sort [created_at desc, name asc]` | `ORDER BY created_at DESC, name ASC` |
| **take** | `take 25` | `LIMIT 25` |
| **skip** | `skip 50` | `OFFSET 50` |

### Parameters

Parameters are auto-extracted from the query and converted to dialect-specific placeholders:

| Syntax | Description | Postgres | SQLite / DuckDB / MySQL |
|:---|:---|:---|:---|
| `$name` | Named parameter | `$1` | `?` |
| `${name}` | Braced parameter | `$1` | `?` |
| `'literal'` | String literal (auto-extracted) | `$1` | `?` |

```
from users | filter email == $email and role == 'admin'
-- Postgres: WHERE (email = $1) AND (role = $2)  → params: ["email", "admin"]
-- SQLite:   WHERE (email = ?) AND (role = ?)    → params: ["email", "admin"]
```

### Expressions & Operators

```
# Comparison
==  !=  <  <=  >  >=

# Logical
and  or  not

# Null checks
is null     is not null

# Membership
in (1, 2, 3)
not in ('a', 'b')

# Subquery
filter id in (from active_users | select [id])

# Arithmetic
+  -  *  /

# Functions
count(*)  sum(amount)  avg(score)  min(x)  max(x)  coalesce(a, b)
```

### Mutations (DML)

#### Insert

```
into users | insert [name = $name, email = $email]
```
→ `INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *;`

#### Update

```
from users | filter id == $id | update [name = $name, email = $email]
```
→ `UPDATE users SET name = $1, email = $2 WHERE (id = $3);`

> ⚠️ `update` **requires** a preceding `filter` stage — PipeQL enforces this to prevent accidental mass updates.

#### Delete

```
from users | filter id == $id | delete
```
→ `DELETE FROM users WHERE (id = $1);`

> ⚠️ `delete` **requires** a preceding `filter` stage — same safety enforcement as `update`.

#### Upsert (Insert or Update on Conflict)

```
into users
| upsert [id = $id, name = $name, email = $email]
| conflict [id]
| do update [name = $name, email = $email]
```

| Dialect | Output |
|:---|:---|
| Postgres / SQLite / DuckDB | `INSERT INTO users (...) VALUES (...) ON CONFLICT (id) DO UPDATE SET name = $4, email = $5;` |
| MySQL | `INSERT INTO users (...) VALUES (...) ON DUPLICATE KEY UPDATE name = VALUES(name), email = VALUES(email);` |

### Union

```
from active_users | select [id, name]
| union
from archived_users | select [id, name]
```
→ `SELECT id, name FROM active_users UNION SELECT id, name FROM archived_users;`

Use `union all` to include duplicates.

### Subqueries

```
from orders
| filter customer_id in (from vip_customers | select [id])
| select [order_id, total]
```
→ `SELECT order_id, total FROM orders WHERE customer_id IN (SELECT id FROM vip_customers);`

### DDL (Table Schema)

```
table users [
  id integer primary_key auto_increment,
  name string not_null,
  email string not_null unique,
  active bool default true,
  created_at timestamp default '2024-01-01'
]
```

| Type | Column Modifiers |
|:---|:---|
| `integer`, `float`, `string`, `bool`, `timestamp` | `primary_key`, `auto_increment`, `not_null`, `unique`, `default <value>` |

### Comments

```
-- This is a line comment
from users | select [id, name]  -- inline comment
```

Comments are preserved in the lossless AST for IDE tooling.

---

## SDK Usage

### Rust

```rust
use pipeql_core::api;

let result = api::compile("from users | filter id == $id | select [name]", "postgres").unwrap();
println!("{}", result.sql);    // SELECT name FROM users WHERE (id = $1);
println!("{:?}", result.params); // ["id"]
```

### JavaScript / TypeScript

```javascript
import { compile } from '@flaxmbot/pipeql';

const { sql, params } = compile(
  "from notes | filter category == $cat | sort [updated_at desc]",
  "sqlite"
);
console.log(sql);    // SELECT * FROM notes WHERE (category = ?) ORDER BY updated_at DESC;
console.log(params); // ["cat"]
```

### Python

```python
import pipeql_python as pipeql

res = pipeql.compile("into users | insert [name = $name, email = $email]", "postgres")
print(res["sql"])    # INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *;
print(res["params"]) # ["name", "email"]
```

### Go

```go
package main

import (
    "fmt"
    "log"
    pipeql "github.com/Flaxmbot/PipeQL/go"
)

func main() {
    res, err := pipeql.Compile("from users | filter age >= $min | select [id, name]", "postgres")
    if err != nil { log.Fatal(err) }
    fmt.Println("SQL:", res.SQL)       // SELECT id, name FROM users WHERE (age >= $1);
    fmt.Println("Params:", res.Params) // ["min"]
}
```

### C

```c
#include <stdio.h>
#include "libpipeql.h"

int main() {
    PipeqlError err = {0};
    PipeqlResult* res = pipeql_compile("from users | filter id == $id", "postgres", &err);
    if (!res) { fprintf(stderr, "Error: %s\n", err.message); return 1; }
    printf("SQL: %s\n", res->sql);
    pipeql_result_free(res);
    return 0;
}
```
```bash
gcc demo.c -I./crates/pipeql-cffi/include -L./target/release -lpipeql_cffi -o demo
```

### CLI

```bash
pipeql compile "from users | take 10" --dialect postgres
pipeql compile "from users | filter id == \$id" --dialect sqlite
pipeql parse "from users | select [id, name]"
pipeql dialects
pipeql version
```

---

## Ecosystem

| Component | Description |
|:---|:---|
| [`pipeql-core`](crates/pipeql-core) | Core compiler: lexer → parser → AST → codegen |
| [`pipeql-cli`](crates/pipeql-cli) | Command-line tool |
| [`pipeql-wasm`](crates/pipeql-wasm) | WebAssembly target for browsers |
| [`pipeql-python`](crates/pipeql-python) | Python binding (PyO3, ABI3) |
| [`pipeql-cffi`](crates/pipeql-cffi) | C ABI shared library |
| [`pipeql-lsp`](crates/pipeql-lsp) | Language Server Protocol |
| [`js/`](js) | JavaScript/TypeScript SDK |
| [`go/`](go) | Go binding (CGO) |
| [`python/`](python) | Python package + driver adapters |
| [`docs-web/`](docs-web) | Interactive documentation + WASM playground |
| [`extensions/`](extensions) | VS Code extension |
| [`tree-sitter-pipeql/`](tree-sitter-pipeql) | Tree-sitter grammar |

---

## Compiler Architecture

```
Source Text ──→ Lexer ──→ Tokens ──→ Parser ──→ AST ──→ Codegen ──→ SQL + Params
                          │                     │                    │
                          │                     │                    ├─ PostgreSQL ($1, $2)
                          │                     │                    ├─ SQLite     (?, ?)
                          │                     │                    ├─ DuckDB     (?, ?)
                          │                     │                    └─ MySQL      (?, ?)
                          │                     │
                          │                     └─ Lossless AST (spans + comments)
                          │
                          └─ Character-level span tracking
```

1. **Lexer** — Hand-written tokenizer with exact character positions for IDE support
2. **Parser** — Pratt parser producing a lossless abstract syntax tree
3. **Codegen** — Walks the AST, extracts all values into bind parameters, emits dialect-specific SQL

All language bindings are thin wrappers calling the Rust core through `pipeql-core`'s API.

**Safety:** `#![deny(unsafe_code)]` enforced across the entire compiler core.

---

## Error Messages

PipeQL provides compiler-grade error messages with exact positions and actionable suggestions:

```
Error at line 1, col 1: Unknown keyword 'selct'
  hint: Did you mean 'select'?

Error at line 1, col 35: 'update' requires a preceding 'filter' stage
  hint: Add a filter to prevent accidental mass updates.
  help: from users | filter id == $id | update [...]

Error at line 1, col 15: Unclosed string literal
  hint: Add a closing single quote (') to terminate the string.
```

Features: Levenshtein-based fuzzy keyword matching, contextual hints, unclosed string/subquery detection, duplicate column detection, empty pipeline errors, filter-before-mutate enforcement.

---

## AI & LLM Integration

PipeQL ships with an optimized **System Prompt** for code generation with LLMs (GPT-4, Claude, Gemini, etc.):

```python
# Python
import pipeql_python
system_prompt = pipeql_python.SYSTEM_PROMPT
```

```javascript
// JavaScript — bundled in npm package
import prompt from '@flaxmbot/pipeql/ai/system_prompt.md';
```

Direct download: [`ai/system_prompt.md`](https://raw.githubusercontent.com/Flaxmbot/PipeQL/master/ai/system_prompt.md)

---

## Contributing

```bash
git clone https://github.com/Flaxmbot/PipeQL.git && cd PipeQL
cargo test --workspace              # Run all tests
cargo bench -p pipeql-core          # Benchmarks
cargo build --release -p pipeql-cli # Build CLI
```

---

## License

[MIT](LICENSE)
