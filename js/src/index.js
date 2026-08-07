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
import init, { compile as wasmCompile, compileWithCatalog as wasmCompileWithCatalog, parseAst, supportedDialects as wasmSupportedDialects, version as wasmVersion, } from "../dist/pipeql_wasm.js";
let initialized = false;
/**
 * Idempotently initialize the WASM module.
 *
 * In the browser the wasm is fetched relative to the module URL. In Node the
 * bytes are read from disk. Safe to call multiple times.
 */
export async function initWasm() {
    if (initialized)
        return;
    const isNode = typeof process !== "undefined" && process.versions?.node != null;
    if (isNode) {
        const { readFileSync } = await import("node:fs");
        const { dirname, join } = await import("node:path");
        const { fileURLToPath } = await import("node:url");
        const here = dirname(fileURLToPath(import.meta.url));
        const bytes = readFileSync(join(here, "../dist/pipeql_wasm_bg.wasm"));
        await init({ module_or_path: bytes });
    }
    else {
        await init();
    }
    initialized = true;
}
function toResult(compiled) {
    const analysis = compiled.analysis;
    const params = compiled.params;
    return {
        sql: compiled.sql,
        params,
        statementType: compiled.statement_type,
        isMutation: compiled.is_mutation,
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
export async function compile(source, dialect = "postgres") {
    await initWasm();
    return toResult(wasmCompile(source, dialect));
}
/**
 * Compile with a schema catalog for column validation. Unknown columns raise.
 */
export async function compileWithCatalog(source, catalog, dialect = "postgres") {
    await initWasm();
    const catalogJson = JSON.stringify({ tables: catalog });
    return toResult(wasmCompileWithCatalog(source, dialect, catalogJson));
}
/**
 * Parse a PipeQL source into a JSON-serializable, lossless AST (spans and
 * comments preserved). Useful for editors and tooling.
 */
export async function parse(source) {
    await initWasm();
    return parseAst(source);
}
/** List of supported target dialects. */
export async function supportedDialects() {
    await initWasm();
    return wasmSupportedDialects();
}
/** PipeQL version. */
export async function version() {
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
export function pipeql(strings, ...values) {
    let source = "";
    strings.forEach((part, i) => {
        source += part;
        if (i < values.length)
            source += `$p${i}`;
    });
    return new PipeqlTemplate(source, values);
}
export class PipeqlTemplate {
    constructor(source, values) {
        this.source = source;
        this.values = values;
    }
    /** Compile to a dialect; `values` carry the bound parameters. */
    async compile(dialect = "postgres") {
        const result = await compile(this.source, dialect);
        return { ...result, values: this.values };
    }
    /** Alias of {@link compile}. */
    for(dialect = "postgres") {
        return this.compile(dialect);
    }
}
