/**
 * @pipeql/js/builder — fluent query builder for PipeQL.
 *
 * Composes a PipeQL **source string** stage by stage, then compiles it through
 * the same WASM facade as any hand-written query — so a builder query and a
 * literal string query are provably identical. No dual parser, no semantic
 * drift.
 *
 * Object inserts/updates (`insert`, `update`, `upsert`, `doUpdate`) accept
 * `{column: value}` objects and auto-generate `$b0`, `$b1`, ... bind
 * parameters — the `$data` ergonomics without a driver.
 *
 * ```ts
 * import { PipeQL } from "@pipeql/js/builder";
 *
 * const q = PipeQL.from("notes")
 *   .filter("is_archived == 0")
 *   .sort(["created_at desc"])
 *   .take(10);
 *
 * q.source();                      // "from notes | filter ... | take 10"
 * const { sql, params } = await q.compile("postgres");
 * q.values;                        // { b0: ..., b1: ... } for object inserts
 *
 * // Runs through any PipeqlDriver (duck-typed):
 * const rows = await db.query(q);
 * ```
 */
import { type CompileResult, type Dialect } from "./index.js";
/** A typed value accepted in object inserts/updates. */
export type BuilderValue = string | number | boolean | null;
export type Cols = string | string[];
/** Something usable as a union operand: a source string or a builder. */
export type UnionOperand = string | PipeQL;
type Assignments = Record<string, BuilderValue> | string | string[];
/**
 * Fluent PipeQL query builder. Every stage method appends to the composed
 * source and returns `this` for chaining.
 */
export declare class PipeQL {
    private _source;
    private _values;
    constructor(source: string);
    /** Start a read pipeline: `from <table>`. */
    static from(table: string): PipeQL;
    /** Start an insert/upsert pipeline: `into <table>`. */
    static into(table: string): PipeQL;
    /** Start from an explicit PipeQL source string. */
    static raw(source: string): PipeQL;
    private _stage;
    /** `| filter <expr>` */
    filter(expr: string): this;
    /** `| select [<cols>]` */
    select(cols: Cols): this;
    /** `| derive [<cols>]` */
    derive(cols: Cols): this;
    /** `| sort [<cols>]` */
    sort(cols: Cols): this;
    /** `| take <n>` */
    take(n: number): this;
    /** `| skip <n>` */
    skip(n: number): this;
    /** `| join <table> on <on>` */
    join(table: string, on: string): this;
    /** `| left join <table> on <on>` */
    leftJoin(table: string, on: string): this;
    /** `| right join <table> on <on>` */
    rightJoin(table: string, on: string): this;
    /** `| full join <table> on <on>` */
    fullJoin(table: string, on: string): this;
    /** `| inner join <table> on <on>` */
    innerJoin(table: string, on: string): this;
    /** `| group [<cols>] (<aggs>)` */
    group(cols: Cols, aggs: string): this;
    /** `| union <other>` where `other` is a source string or builder. */
    union(other: UnionOperand): this;
    /** `| union all <other>` */
    unionAll(other: UnionOperand): this;
    /** Append an explicit stage string. */
    rawStage(stage: string): this;
    /** `| insert [...]` with auto-generated `$b0, $b1, ...` params. */
    insert(values: Assignments): this;
    /** `| update [...]` (requires a preceding filter stage). */
    update(values: Assignments): this;
    /** `| update all [...]` — explicit opt-in for a full-table update that
     * bypasses the filter guard. */
    updateAll(values: Assignments): this;
    /** `| delete` */
    delete(): this;
    /** `| delete all` — explicit opt-in for a full-table delete that bypasses
     * the filter guard. */
    deleteAll(): this;
    /** `| upsert [...]` */
    upsert(values: Assignments): this;
    /** `| conflict [<cols>]` */
    conflict(cols: Cols): this;
    /** `| do update [...]` */
    doUpdate(values: Assignments): this;
    private _assignments;
    /** The composed PipeQL source string. */
    source(): string;
    /** Bound values from object inserts/updates, keyed by `$bN` name. */
    get values(): Record<string, BuilderValue>;
    /** Compile through the standard facade. */
    compile(dialect?: Dialect): Promise<CompileResult & {
        values: Record<string, BuilderValue>;
    }>;
    toString(): string;
}
export {};
