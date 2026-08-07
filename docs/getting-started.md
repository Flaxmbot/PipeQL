# Getting Started with PipeQL

## Install

### CLI

```bash
cargo build --release -p pipeql-cli
# binary: target/release/pipeql[.exe]
```

### Library (Rust)

```toml
[dependencies]
pipeql-core = { path = "crates/pipeql-core" }
```

## Your first query

```bash
pipeql compile "from users | filter age >= $min | select [id, name] | sort [name asc] | take 10"
```

```sql
SELECT id, name FROM users
WHERE (age >= $1)
ORDER BY name ASC
LIMIT 10;
```

The `$min` value is **not** inserted into SQL — it is returned in a parameter
vector you bind separately. Injection is impossible by construction.

## Bind parameters

```bash
pipeql compile --json "from t | filter id == $id or name == 'admin'"
```

```json
{
  "sql": "SELECT * FROM t\nWHERE ((id = $1) OR (name = $2));",
  "params": ["id", "admin"],
  "dialect": "postgres",
  "statement_type": "select",
  "is_mutation": false,
  "parameter_count": 2
}
```

String literals like `'admin'` are extracted too — the same anti-injection
discipline applies to literals, not just `$params`.

## Dialects

```bash
pipeql compile --dialect sqlite "from users | take 5"
pipeql compile --dialect duckdb "from parquet | select [col_a]"
pipeql compile --dialect mysql  "from orders | skip 20 | take 10"
```

| Dialect  | Placeholder | Notes                    |
| -------- | ----------- | ------------------------ |
| postgres | `$1`, `$2`  | dedup by param name      |
| sqlite   | `?`         | one per occurrence       |
| duckdb   | `?`         | one per occurrence       |
| mysql    | `?`         | one per occurrence       |

## Next steps

- [Language Specification](SPEC.md)
- [Bindings](bindings.md) — use PipeQL from JS/WASM, Python, C, or Go.
- [Examples](../examples/) — realistic queries with generated SQL.
