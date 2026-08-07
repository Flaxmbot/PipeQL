/*
 * libpipeql.h — C API for PipeQL.
 *
 * Pipelined, injection-safe polyglot query language. Compiles PipeQL source
 * to target-dialect SQL with a fully isolated parameter map.
 *
 * Memory model:
 *   - Every `char*` returned by PipeQL is heap-allocated and owned by the
 *     caller. Release it with the matching `pipeql_*_free` function.
 *   - `PipeqlError` must be zero-initialized by the caller; any message it
 *     holds afterwards is owned and must be released with `pipeql_error_clear`.
 *
 * Thread safety: all functions are thread-safe. A result or error may only be
 * used on the thread that created it.
 *
 * Example:
 *   PipeqlError err = {0};
 *   PipeqlResult* res = pipeql_compile(
 *       "from users | filter age >= $min | select [id, name]", "postgres", &err);
 *   if (!res) { fprintf(stderr, "error: %s\n", err.message); pipeql_error_clear(&err); return 1; }
 *   printf("%s\n", res->sql);
 *   pipeql_result_free(res);
 */

#ifndef PIPEQL_H
#define PIPEQL_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>

/*
 * PipeQL builds its C library as a Rust cdylib, which exports all
 * `#[no_mangle] extern "C"` symbols directly. No `dllimport`/`dllexport`
 * decoration is therefore required: link against the produced import library
 * (MSVC: pipeql_cffi.dll.lib) or the DLL itself (MinGW: pipeql_cffi.dll).
 */
#define PIPEQL_API

/* Error kinds. */
#define PIPEQL_ERR_NONE 0
#define PIPEQL_ERR_PARSE 1
#define PIPEQL_ERR_ANALYSIS 2
#define PIPEQL_ERR_CODEGEN 3

typedef struct PipeqlResult {
    char* sql;           /* target-dialect SQL (owned) */
    char* params_json;   /* JSON array of parameter names, e.g. ["min_age"] (owned) */
    char* statement_type;/* "select"|"insert"|"update"|"delete"|"create_table"|"upsert"|"union" (owned) */
    int is_mutation;     /* non-zero for insert/update/delete/upsert */
    char* analysis_json; /* full analysis document: param map, types, occurrences (owned) */
    int parameter_count; /* number of extracted parameters */
} PipeqlResult;

typedef struct PipeqlError {
    int kind;           /* one of PIPEQL_ERR_* */
    char* message;      /* human-readable message (owned) */
} PipeqlError;

/* Compile a PipeQL source string into target-dialect SQL.
 *
 *   source  — NUL-terminated PipeQL source.
 *   dialect — NUL-terminated dialect name ("postgres" default, "sqlite",
 *             "duckdb", "mysql"). May be NULL to use the default.
 *   err     — caller-owned, zero-initialized error slot.
 *
 * Returns a heap-allocated PipeqlResult on success (free with
 * pipeql_result_free), or NULL on failure with *err populated.
 */
PIPEQL_API PipeqlResult* pipeql_compile(const char* source, const char* dialect,
                                        PipeqlError* err);

/* Compile a PipeQL source string, optionally validating columns against a
 * JSON schema catalog.
 *
 *   source       — NUL-terminated PipeQL source.
 *   dialect      — NUL-terminated dialect name. May be NULL for default.
 *   catalog_json — NUL-terminated JSON catalog string, or NULL for no
 *                  validation. Format:
 *                  {"tables":{"users":{"name":"users","columns":[{"name":"id",
 *                  "ty":"Integer"}]}}}
 *   err          — caller-owned, zero-initialized error slot.
 *
 * Returns a heap-allocated PipeqlResult on success (free with
 * pipeql_result_free), or NULL on failure with *err populated.
 */
PIPEQL_API PipeqlResult* pipeql_compile_with_catalog(
    const char* source, const char* dialect, const char* catalog_json,
    PipeqlError* err);

/* Parse a PipeQL source into a lossless statement AST, returned as a JSON
 * string. Covers read pipelines, inserts, upserts, unions, and DDL.
 *
 * The returned string must be freed with pipeql_string_free.
 *
 *   source — NUL-terminated PipeQL source.
 *   err    — caller-owned, zero-initialized error slot.
 *
 * Returns a heap-allocated JSON string on success (free with
 * pipeql_string_free), or NULL on failure with *err populated.
 */
PIPEQL_API char* pipeql_parse(const char* source, PipeqlError* err);

/* Return the list of supported dialect names as a JSON array string,
 * e.g. ["postgres","sqlite","duckdb","mysql"].
 *
 * The returned string must be freed with pipeql_string_free.
 */
PIPEQL_API char* pipeql_supported_dialects(void);

/* Return the PipeQL version as a static string. Never free. */
PIPEQL_API const char* pipeql_version(void);

/* Free a result from pipeql_compile or pipeql_compile_with_catalog.
 * Passing NULL is a no-op.
 */
PIPEQL_API void pipeql_result_free(PipeqlResult* res);

/* Free a string returned by pipeql_parse or pipeql_supported_dialects.
 * Passing NULL is a no-op.
 */
PIPEQL_API void pipeql_string_free(char* s);

/* Free any message held by *err and reset it. */
PIPEQL_API void pipeql_error_clear(PipeqlError* err);

#ifdef __cplusplus
}
#endif

#endif /* PIPEQL_H */
