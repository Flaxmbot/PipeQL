/**
 * @pipeql/js — TypeScript SDK for PipeQL.
 *
 * The SDK wraps the WASM core and exposes:
 *  - `compile(source, dialect?)`       — compile to SQL + parameter map
 *  - `compileWithCatalog(...)`         — compile with schema validation
 *  - `pipeql` tagged template          — ergonomic, injection-safe interpolation
 *  - `parse(source)`                   — lossless AST for tooling
 *  - `supportedDialects()`, `version()`
 */

import init, {
  compile as wasmCompile,
  compileWithCatalog as wasmCompileWithCatalog,
  parseAst,
  supportedDialects as wasmSupportedDialects,
  version as wasmVersion,
  type Compiled,
} from "../dist/pipeql_wasm.js";

export type Dialect = "postgres" | "sqlite" | "duckdb" | "mysql";

/** The kind of statement a compiled query represents. */
export type StatementType =
  | "select"
  | "insert"
  | "update"
  | "delete"
  | "create_table"
  | "upsert"
  | "union";

export interface ParamMeta {
  name: string;
  ty: string;
  occurrences: number[];
}

export interface Analysis {
  param_map: ParamMeta[];
  validated_columns: boolean;
}

export interface CompileResult {
  /** Target-dialect SQL with positional placeholders (`$1`, `?`, ...). */
  sql: string;
  /** Ordered parameter names (bind values in this order). */
  params: string[];
  /** Statement kind, so you can dispatch `.all()` vs `.run()` without parsing SQL. */
  statementType: StatementType;
  /** True for mutations (insert/update/delete). */
  isMutation: boolean;
  /** Full semantic analysis (param map, types, occurrences). */
  analysis: Analysis;
  /** Number of distinct parameters. */
  parameterCount: number;
}

export interface SchemaColumn {
  name: string;
  ty: "Integer" | "Float" | "String" | "Bool" | "Null" | "Any";
}

export interface SchemaTable {
  name: string;
  columns: SchemaColumn[];
}

export type Catalog = Record<string, SchemaTable>;

let initialized = false;

/**
 * Idempotently initialize the WASM module.
 *
 * In the browser the wasm is fetched relative to the module URL. In Node the
 * bytes are read from disk. Safe to call multiple times.
 */
export async function initWasm(): Promise<void> {
  if (initialized) return;
  const isNode =
    typeof process !== "undefined" && process.versions?.node != null;
  if (isNode) {
    const { readFileSync } = await import("node:fs");
    const { dirname, join } = await import("node:path");
    const { fileURLToPath } = await import("node:url");
    const here = dirname(fileURLToPath(import.meta.url));
    const bytes = readFileSync(join(here, "../dist/pipeql_wasm_bg.wasm"));
    await init({ module_or_path: bytes });
  } else {
    await init();
  }
  initialized = true;
}

function toResult(compiled: Compiled): CompileResult {
  const analysis = compiled.analysis as Analysis;
  const params = compiled.params as unknown as string[];
  return {
    sql: compiled.sql,
    params,
    statementType: compiled.statement_type as StatementType,
    isMutation: compiled.is_mutation as boolean,
    analysis,
    parameterCount: params.length,
  };
}

/**
 * Compile a PipeQL source string for a target dialect.
 *
 * ```ts
 * const { sql, params } = await compile(
 *   "from users | filter age >= $min | select [id, name]",
 *   "postgres",
 * );
 * // sql:    "SELECT id, name FROM users\nWHERE (age >= $1);"
 * // params: ["min"]
 * ```
 */
export async function compile(
  source: string,
  dialect: Dialect = "postgres",
): Promise<CompileResult> {
  await initWasm();
  return toResult(wasmCompile(source, dialect));
}

/**
 * Compile with a schema catalog for column validation. Unknown columns raise.
 */
export async function compileWithCatalog(
  source: string,
  catalog: Catalog,
  dialect: Dialect = "postgres",
): Promise<CompileResult> {
  await initWasm();
  const catalogJson = JSON.stringify({ tables: catalog });
  return toResult(wasmCompileWithCatalog(source, dialect, catalogJson));
}

/**
 * Parse a PipeQL source into a JSON-serializable, lossless AST (spans and
 * comments preserved). Useful for editors and tooling.
 */
export async function parse(source: string): Promise<unknown> {
  await initWasm();
  return parseAst(source);
}

/** List of supported target dialects. */
export async function supportedDialects(): Promise<Dialect[]> {
  await initWasm();
  return wasmSupportedDialects() as unknown as Dialect[];
}

/** PipeQL version. */
export async function version(): Promise<string> {
  await initWasm();
  return wasmVersion();
}

/**
 * Tagged template for ergonomic, injection-safe queries. Interpolated values
 * become named bind parameters (`p0`, `p1`, ...), never inlined SQL.
 *
 * ```ts
 * const q = pipeql`from users | filter age >= ${18} and plan == ${"pro"} | select [id]`;
 * const { sql, params, values } = await q.compile("postgres");
 * // sql:    "SELECT id FROM users\nWHERE ((age >= $1) AND (plan = $2));"
 * // params: ["p0", "p1"]
 * // values: [18, "pro"]
 * ```
 */
export function pipeql(
  strings: TemplateStringsArray,
  ...values: unknown[]
): PipeqlTemplate {
  let source = "";
  strings.forEach((part, i) => {
    source += part;
    if (i < values.length) source += `$p${i}`;
  });
  return new PipeqlTemplate(source, values);
}

export class PipeqlTemplate {
  /** Raw interpolated values in placeholder order. */
  readonly values: unknown[];

  /** Build the PipeQL source (each interpolation replaced by `$pN`). */
  readonly source: string;

  constructor(source: string, values: unknown[]) {
    this.source = source;
    this.values = values;
  }

  /** Compile to a dialect; `values` carry the bound parameters. */
  async compile(dialect: Dialect = "postgres"): Promise<CompileResult & { values: unknown[] }> {
    const result = await compile(this.source, dialect);
    return { ...result, values: this.values };
  }

  /** Alias of {@link compile}. */
  for(dialect: Dialect = "postgres"): ReturnType<PipeqlTemplate["compile"]> {
    return this.compile(dialect);
  }
}
