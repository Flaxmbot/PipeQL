/* c_demo.c — demonstrates libpipeql from C. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "libpipeql.h"

static int check(const char* label, int cond) {
    if (!cond) {
        fprintf(stderr, "FAIL: %s\n", label);
        return 1;
    }
    printf("ok: %s\n", label);
    return 0;
}

int main(void) {
    int failures = 0;

    /* 1. Basic compile with parameter extraction. */
    PipeqlError err = {0};
    PipeqlResult* res = pipeql_compile(
        "from users | filter age >= $min and status == 'active' "
        "| select [id, name] | sort [name asc] | take 10",
        "postgres", &err);
    if (!res) {
        fprintf(stderr, "compile failed: %s\n", err.message);
        pipeql_error_clear(&err);
        return 1;
    }
    failures += check("sql has SELECT", strstr(res->sql, "SELECT id, name FROM users") != NULL);
    failures += check("sql has param placeholder", strstr(res->sql, "$1") != NULL);
    failures += check("params json lists min",
                      strstr(res->params_json, "min") != NULL);
    failures += check("statement type is select",
                      res->statement_type != NULL && strcmp(res->statement_type, "select") == 0);
    failures += check("select is not a mutation", res->is_mutation == 0);
    printf("--- sql ---\n%s\n--- params ---\n%s\n", res->sql, res->params_json);
    pipeql_result_free(res);

    /* 1b. Mutation metadata drives driver dispatch. */
    PipeqlError errm = {0};
    PipeqlResult* ins = pipeql_compile(
        "into notes | insert [title = $title, is_pinned = 0]", "sqlite", &errm);
    if (!ins) {
        fprintf(stderr, "insert compile failed: %s\n", errm.message);
        pipeql_error_clear(&errm);
        return 1;
    }
    failures += check("statement type is insert",
                      ins->statement_type != NULL && strcmp(ins->statement_type, "insert") == 0);
    failures += check("insert is a mutation", ins->is_mutation != 0);
    pipeql_result_free(ins);

    /* 2. Dialect selection. */
    PipeqlError err2 = {0};
    PipeqlResult* sqlite = pipeql_compile("from t | filter id == $id | take 5", "sqlite", &err2);
    if (!sqlite) {
        fprintf(stderr, "sqlite compile failed: %s\n", err2.message);
        pipeql_error_clear(&err2);
        return 1;
    }
    failures += check("sqlite uses ? placeholder", strchr(sqlite->sql, '?') != NULL);
    pipeql_result_free(sqlite);

    /* 3. Error path: parse failure with message. */
    PipeqlError err3 = {0};
    PipeqlResult* bad = pipeql_compile("from users | explode", "postgres", &err3);
    failures += check("bad query returns NULL", bad == NULL);
    failures += check("error kind is parse", err3.kind == PIPEQL_ERR_PARSE);
    failures += check("error message non-empty", err3.message != NULL && strlen(err3.message) > 0);
    if (err3.message) fprintf(stderr, "expected error: %s\n", err3.message);
    pipeql_error_clear(&err3);

    /* 4. NULL-source guard. */
    PipeqlError err4 = {0};
    PipeqlResult* nullsrc = pipeql_compile(NULL, "postgres", &err4);
    failures += check("NULL source returns NULL", nullsrc == NULL);
    pipeql_error_clear(&err4);

    /* 5. Version. */
    const char* v = pipeql_version();
    failures += check("version non-empty", v != NULL && strlen(v) > 0);
    printf("pipeql version: %s\n", v);

    if (failures == 0) {
        printf("\nAll C demo checks passed.\n");
        return 0;
    }
    printf("\n%d check(s) failed.\n", failures);
    return 1;
}
