/**
 * @pipeql/js/driver — zero-boilerplate PipeQL database adapters.
 *
 * Wraps any native Node database connection and auto-handles:
 *  - compilation (PipeQL source -> dialect SQL)
 *  - parameter binding (named PipeQL params -> positional driver args)
 *  - dispatch (SELECT via `.all()`/`.query()`, mutations via `.run()`)
 *  - `$data` object expansion (partial insert/update with zero boilerplate)
 *  - `insertAndFetch` / `updateAndFetch` (single-call write + return)
 *
 * Supported drivers (auto-detected by duck-typing, or forced via `driver`):
 *  `better-sqlite3`, `sqlite3`, `pg`, `postgres.js`, `mysql2`, `duckdb`
 *
 * ```ts
 * import { createPipeqlDriver } from "@pipeql/js/driver";
 * const db = createPipeqlDriver(rawConnection, { dialect: "sqlite" });
 *
 * const rows = await db.query("from notes | filter category == $cat", { cat: "Ideas" });
 * const { lastId, changes } = await db.execute(
 *   "into notes | insert [title = $title]", { title: "New Note" });
 * const note = await db.insertAndFetch("into notes | insert $data", req.body);
 * const updated = await db.updateAndFetch(
 *   "from notes | filter id == $id | update $data", { id, data: req.body });
 * const result = await db.pipeql`from notes | filter title == ${userInput}`;
 * ```
 */
import { type CompileResult, type Dialect } from "./index.js";
export type DriverKind = "better-sqlite3" | "sqlite3" | "pg" | "postgres.js" | "mysql2" | "duckdb";
export interface DriverOptions {
    /** Target dialect. Inferred from the driver kind when omitted. */
    dialect?: Dialect;
    /** Force a driver kind instead of duck-typed auto-detection. */
    driver?: DriverKind;
}
export type ParamValues = Record<string, unknown>;
/** Result shape for mutations: affected-row metadata plus an empty rows array. */
export type RunResult = {
    lastId?: unknown;
    changes?: unknown;
    rows: [];
};
/** Return type of `query()`/tagged template: rows for selects, run metadata for mutations. */
export type QueryReturn<T = unknown> = T[] | RunResult;
/** Return type of `insertAndFetch()`/`updateAndFetch()`: the affected row(s) or run metadata. */
export type FetchReturn<T = unknown> = T | T[] | {
    lastId?: unknown;
    changes?: number;
    rows: [];
};
export declare class PipeqlDriver {
    /** Detected (or forced) driver kind. */
    readonly driver: DriverKind;
    /** Target compile dialect. */
    readonly dialect: Dialect;
    private readonly conn;
    private readonly cache;
    private readonly mysqlPromise;
    constructor(conn: unknown, options?: DriverOptions);
    /**
     * Resolve `$data` object expansion. When the source references `$data`, the
     * data object's keys are expanded into explicit column assignments and its
     * values become bound params. `wholeObjectAsData` treats the entire params
     * object as the data object (used by `insertAndFetch`).
     */
    private prepare;
    /** Compile (cached per source) to the driver dialect. */
    private compiled;
    /** Map named PipeQL params to positional driver args (literals bind as themselves). */
    private bind;
    /** Run a row-returning statement (SELECT or `... RETURNING *`) and fetch rows. */
    private runQuery;
    private runRaw;
    /**
     * Compile source and bind params. Returns the compiler result plus the
     * positional `args` array ready for the native driver.
     */
    compile(source: string, params?: ParamValues): Promise<CompileResult & {
        args: unknown[];
    }>;
    /**
     * Run any statement, auto-dispatching `.all()` vs `.run()`. Returns rows for
     * selects; mutations return `{ lastId, changes, rows: [] }`.
     */
    query<T = unknown>(source: string, params?: ParamValues): Promise<QueryReturn<T>>;
    /**
     * Run a statement and return its execution result: mutations return
     * `{ lastId, changes, rows: [] }`, selects return `{ rows }`.
     */
    execute(source: string, params?: ParamValues): Promise<RunResult | {
        rows: unknown[];
    }>;
    /**
     * Single-call insert + return. Inserts the record and returns the created
     * row(s) using `RETURNING *` (sqlite, postgres, duckdb). On mysql2 — which
     * lacks `RETURNING` — falls back to run metadata `{ lastId, changes, rows: [] }`.
     *
     * With `$data`, the entire params object is treated as the data object:
     * ```ts
     * const note = await db.insertAndFetch("into notes | insert $data", req.body);
     * ```
     */
    insertAndFetch<T = Record<string, unknown>>(source: string, params?: ParamValues): Promise<FetchReturn<T>>;
    /**
     * Single-call update + return. Updates matching rows and returns the updated
     * row(s) via `RETURNING *` (sqlite, postgres, duckdb). On mysql2 falls back
     * to run metadata `{ lastId, changes, rows: [] }`.
     *
     * ```ts
     * const note = await db.updateAndFetch(
     *   "from notes | filter id == $id | update $data", { id, data: req.body });
     * ```
     */
    updateAndFetch<T = Record<string, unknown>>(source: string, params?: ParamValues): Promise<FetchReturn<T>>;
    /**
     * Tagged template: interpolated values become named bind params (never
     * inlined SQL). Dispatches like {@link query}.
     *
     * ```ts
     * const rows = await db.pipeql`from notes | filter title == ${userInput}`;
     * ```
     */
    pipeql<T = unknown>(strings: TemplateStringsArray, ...values: unknown[]): Promise<QueryReturn<T>>;
    /** Best-effort close of the underlying connection when it exposes close/end. */
    close(): Promise<void>;
}
/** Wrap any native database connection with automatic PipeQL compilation and dispatch. */
export declare function createPipeqlDriver(conn: unknown, options?: DriverOptions): PipeqlDriver;
