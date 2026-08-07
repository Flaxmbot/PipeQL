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
    char* statement_type;/* "select" | "insert" | "update" | "delete" | "create_table" (owned) */
    int is_mutation;     /* non-zero for insert/update/delete */
    char* analysis_json; /* full analysis document: param map, types, occurrences (owned) */
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

/* Return the PipeQL version as a static string. Never free. */
PIPEQL_API const char* pipeql_version(void);

/* Free a result from pipeql_compile. Passing NULL is a no-op. */
PIPEQL_API void pipeql_result_free(PipeqlResult* res);

/* Free any message held by *err and reset it. */
PIPEQL_API void pipeql_error_clear(PipeqlError* err);

#ifdef __cplusplus
}
#endif

#endif /* PIPEQL_H */
