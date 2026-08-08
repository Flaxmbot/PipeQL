/* bench_c.c — PipeQL C-FFI benchmark
 *
 * Measures compile latency for 1,000 read queries + 1,000 mutations
 * using the C FFI directly. Includes FFI overhead.
 *
 * Build (Windows/MSVC):
 *   cl /O2 bench_c.c /I..\include /I..\target\release /link /LIBPATH:..\target\release pipeql_cffi.dll.lib
 *
 * Build (Linux/macOS):
 *   cc -O2 bench_c.c -I../include -L../target/release -lpipeql_cffi -o bench_c
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "libpipeql.h"

static const char* READ_SHAPES[] = {
    "from orders | join customers on orders.customer_id == customers.id | filter orders.status == 'active' and orders.total >= $min_total | group [customer_id, region] (total = sum(orders.total), cnt = count(*)) | filter total > $threshold | select [customer_id, region, total, cnt] | sort [total desc, customer_id asc] | take 10",
    "from events | filter event_type in ['click', 'view', 'hover'] and user_id is not null | group [user_id, day] (views = count(*), last = max(timestamp)) | sort [views desc] | take 100 | select [user_id, day, views, last]",
    "from users u | join orders o on u.id == o.user_id | join payments p on o.id == p.order_id | filter u.age >= $min and u.age <= $max and p.amount > 0 | select [u.id, u.name, o.total, p.amount] | sort [o.total desc] | take 10",
    "from log_entries | filter level in ['error', 'warn'] and not (service == 'health') | group [service, date] (errors = count(*)) | filter errors > $min_errors | select [service, date, errors] | sort [date desc, errors desc] | take 50",
    "from products | filter category == 'electronics' | filter price >= $min_price and price <= $max_price | select [id, name, price] | sort [price asc] | take 20 | skip 40",
    "from sessions s | join users u on s.user_id == u.id | filter u.plan in ['pro', 'enterprise'] and s.duration >= $min_seconds | group [u.id] (total_duration = sum(s.duration), sessions = count(*)) | select [u.id, u.name, total_duration, sessions] | sort [total_duration desc] | take 10",
    "from inventory | filter stock <= $reorder_point and (warehouse == 'north' or warehouse == 'east') | select [sku, name, stock, warehouse] | sort [stock asc, warehouse asc]",
    "from transactions t | join accounts a on t.account_id == a.id | filter t.status == 'completed' and a.balance >= $min_balance | group [a.country] (revenue = sum(t.amount), count = count(*)) | filter count >= $min_count | select [a.country, revenue, count] | sort [revenue desc] | take 100",
};

static const char* MUTATION_SHAPES[] = {
    "into notes | insert [title = $title, content = $content, category = 'Personal', is_pinned = 0, created_at = CURRENT_TIMESTAMP]",
    "from notes | filter is_archived == 0 | filter id == $id | update [title = $title, is_pinned = 1, updated_at = CURRENT_TIMESTAMP]",
    "from notes | filter id == $id or is_archived == 1 | delete",
};

#define NUM_READ_SHAPES 8
#define NUM_MUTATION_SHAPES 3
#define CORPUS_SIZE 1000

#ifdef _WIN32
#include <windows.h>
static double now_ms(void) {
    static LARGE_INTEGER freq = {0};
    LARGE_INTEGER t;
    if (!freq.QuadPart) QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t);
    return (double)t.QuadPart / (double)freq.QuadPart * 1000.0;
}
#else
static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1e6;
}
#endif

static void build_corpus(char** out, int n) {
    int total_shapes = NUM_READ_SHAPES + NUM_MUTATION_SHAPES;
    for (int i = 0; i < n; i++) {
        const char* shape;
        if (i % total_shapes < NUM_READ_SHAPES) {
            shape = READ_SHAPES[i % NUM_READ_SHAPES];
        } else {
            shape = MUTATION_SHAPES[(i - NUM_READ_SHAPES) % NUM_MUTATION_SHAPES];
        }
        /* Simple copy; parametrization handled via snprintf for read shapes */
        size_t len = strlen(shape) + 32;
        out[i] = (char*)malloc(len);
        if (i % total_shapes < NUM_READ_SHAPES && i % total_shapes == 2) {
            snprintf(out[i], len, "%s", shape); /* shape 2 uses $min/$max already */
        } else {
            snprintf(out[i], len, "%s", shape);
        }
    }
}

static void build_mutation_corpus(char** out, int n) {
    for (int i = 0; i < n; i++) {
        size_t len = strlen(MUTATION_SHAPES[i % NUM_MUTATION_SHAPES]) + 1;
        out[i] = (char*)malloc(len);
        strcpy(out[i], MUTATION_SHAPES[i % NUM_MUTATION_SHAPES]);
    }
}

static void free_corpus(char** corpus, int n) {
    for (int i = 0; i < n; i++) free(corpus[i]);
    free(corpus);
}

typedef struct {
    double total_ms;
    double mean_ms;
    double median_ms;
    double min_ms;
    double max_ms;
    double p95_ms;
    double p99_ms;
} BenchResult;

static int cmp_double(const void* a, const void* b) {
    double da = *(const double*)a;
    double db = *(const double*)b;
    return (da > db) - (da < db);
}

static BenchResult benchmark_corpus(char** queries, int n) {
    double* times = (double*)malloc(n * sizeof(double));
    BenchResult r = {0};

    /* Warmup */
    PipeqlError err = {0};
    PipeqlResult* warmup = pipeql_compile(queries[0], "postgres", &err);
    if (warmup) pipeql_result_free(warmup);
    else pipeql_error_clear(&err);

    /* Total time pass */
    double start = now_ms();
    for (int i = 0; i < n; i++) {
        PipeqlError e = {0};
        PipeqlResult* res = pipeql_compile(queries[i], "postgres", &e);
        if (res) pipeql_result_free(res);
        else pipeql_error_clear(&e);
    }
    double end = now_ms();
    r.total_ms = end - start;

    /* Per-query times for percentiles */
    for (int i = 0; i < n; i++) {
        double s = now_ms();
        PipeqlError e = {0};
        PipeqlResult* res = pipeql_compile(queries[i], "postgres", &e);
        double elapsed = now_ms() - s;
        if (res) pipeql_result_free(res);
        else pipeql_error_clear(&e);
        times[i] = elapsed;
    }

    qsort(times, n, sizeof(double), cmp_double);
    r.mean_ms = r.total_ms / n;
    r.min_ms = times[0];
    r.max_ms = times[n - 1];
    r.median_ms = times[n / 2];
    r.p95_ms = times[(int)(n * 0.95)];
    r.p99_ms = times[(int)(n * 0.99)];

    free(times);
    return r;
}

int main(void) {
    printf("PipeQL C-FFI Benchmark\n");
    printf("========================================\n");
    printf("Version: %s\n\n", pipeql_version());

    /* Build corpora */
    char** read_corpus = (char**)malloc(CORPUS_SIZE * sizeof(char*));
    char** mutation_corpus = (char**)malloc(CORPUS_SIZE * sizeof(char*));
    build_corpus(read_corpus, CORPUS_SIZE);
    build_mutation_corpus(mutation_corpus, CORPUS_SIZE);

    /* Benchmark read corpus */
    printf("--- Read Corpus (1000 queries) ---\n");
    BenchResult read_r = benchmark_corpus(read_corpus, CORPUS_SIZE);
    printf("  Total:   %.2f ms\n", read_r.total_ms);
    printf("  Mean:    %.3f ms/query\n", read_r.mean_ms);
    printf("  Median:  %.3f ms/query\n", read_r.median_ms);
    printf("  Min:     %.3f ms\n", read_r.min_ms);
    printf("  Max:     %.3f ms\n", read_r.max_ms);
    printf("  P95:     %.3f ms\n", read_r.p95_ms);
    printf("  P99:     %.3f ms\n", read_r.p99_ms);

    /* Benchmark mutation corpus */
    printf("\n--- Mutation Corpus (1000 queries) ---\n");
    BenchResult mut_r = benchmark_corpus(mutation_corpus, CORPUS_SIZE);
    printf("  Total:   %.2f ms\n", mut_r.total_ms);
    printf("  Mean:    %.3f ms/query\n", mut_r.mean_ms);
    printf("  Median:  %.3f ms/query\n", mut_r.median_ms);
    printf("  Min:     %.3f ms\n", mut_r.min_ms);
    printf("  Max:     %.3f ms\n", mut_r.max_ms);
    printf("  P95:     %.3f ms\n", mut_r.p95_ms);
    printf("  P99:     %.3f ms\n", mut_r.p99_ms);

    /* Single largest */
    printf("\n--- Single Largest Query ---\n");
    double s = now_ms();
    PipeqlError err = {0};
    PipeqlResult* res = pipeql_compile(read_corpus[0], "postgres", &err);
    double single_ms = now_ms() - s;
    if (res) pipeql_result_free(res);
    else pipeql_error_clear(&err);
    printf("  Time: %.3f ms\n", single_ms);

    /* Summary */
    printf("\n=== Summary ===\n");
    printf("  Read avg:     %.3f ms/query\n", read_r.mean_ms);
    printf("  Mutation avg: %.3f ms/query\n", mut_r.mean_ms);

    free_corpus(read_corpus, CORPUS_SIZE);
    free_corpus(mutation_corpus, CORPUS_SIZE);

    return 0;
}
