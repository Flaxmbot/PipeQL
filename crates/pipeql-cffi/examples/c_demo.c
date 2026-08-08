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

    /* 5. Fluent query builder. */
    PipeqlError err5 = {0};
    PipeqlQuery* q = pipeql_query_from("notes");
    q = pipeql_query_filter(q, "is_archived == 0");
    q = pipeql_query_sort(q, "created_at desc");
    q = pipeql_query_take(q, 10);
    char* src = pipeql_query_source(q);
    failures += check("builder source",
                      src != NULL && strcmp(src, "from notes | filter is_archived == 0 | sort [created_at desc] | take 10") == 0);
    printf("--- builder source ---\n%s\n", src ? src : "(null)");
    pipeql_string_free(src);

    PipeqlResult* built = pipeql_query_compile(q, "postgres", &err5);
    failures += check("builder compiles", built != NULL);
    if (built) {
        failures += check("builder sql has WHERE", strstr(built->sql, "WHERE") != NULL);
        pipeql_result_free(built);
    } else {
        fprintf(stderr, "builder compile failed: %s\n", err5.message);
        pipeql_error_clear(&err5);
    }
    pipeql_query_free(q);

    /* 5b. Builder upsert chain. */
    PipeqlError err5b = {0};
    PipeqlQuery* uq = pipeql_query_into("users");
    uq = pipeql_query_upsert(uq, "id = $id, name = $name");
    uq = pipeql_query_conflict(uq, "id");
    uq = pipeql_query_do_update(uq, "name = $name");
    PipeqlResult* ures = pipeql_query_compile(uq, "postgres", &err5b);
    failures += check("builder upsert compiles", ures != NULL);
    if (ures) {
        failures += check("builder upsert statement type",
                          ures->statement_type != NULL && strcmp(ures->statement_type, "upsert") == 0);
        pipeql_result_free(ures);
    } else {
        fprintf(stderr, "builder upsert failed: %s\n", err5b.message);
        pipeql_error_clear(&err5b);
    }
    pipeql_query_free(uq);

    /* 5c. NULL string args: explicit signal, never a silent empty fragment. */
    failures += check("query_from(NULL) is NULL", pipeql_query_from(NULL) == NULL);
    failures += check("query_into(NULL) is NULL", pipeql_query_into(NULL) == NULL);
    failures += check("query_raw(NULL) is NULL", pipeql_query_raw(NULL) == NULL);

    PipeqlQuery* nq = pipeql_query_from("notes");
    failures += check("query_filter(NULL expr) is NULL", pipeql_query_filter(nq, NULL) == NULL);
    failures += check("query_select(NULL cols) is NULL", pipeql_query_select(nq, NULL) == NULL);
    failures += check("query_join(NULL table) is NULL", pipeql_query_join(nq, NULL, "a.id = b.id") == NULL);
    failures += check("query_join(NULL on) is NULL", pipeql_query_join(nq, "tags", NULL) == NULL);
    failures += check("query_group(NULL aggs) is NULL", pipeql_query_group(nq, "category", NULL) == NULL);
    failures += check("query_update(NULL) is NULL", pipeql_query_update(nq, NULL) == NULL);

    /* The builder must survive untouched and still be owned by us. */
    char* nsrc = pipeql_query_source(nq);
    failures += check("builder untouched after NULL stages",
                      nsrc != NULL && strcmp(nsrc, "from notes") == 0);
    pipeql_string_free(nsrc);
    pipeql_query_free(nq);

    /* 6. Version. */
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
