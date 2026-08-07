use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use pipeql_core::api::compile;

#[path = "corpus.rs"]
mod corpus;

fn bench_transpile(c: &mut Criterion) {
    let queries = corpus::queries();
    assert_eq!(queries.len(), 1_000);
    // Sanity: every corpus query must compile, or the benchmark is meaningless.
    for q in &queries {
        compile(q, "postgres").unwrap_or_else(|e| panic!("corpus query failed: {e}"));
    }

    let mutations = corpus::mutation_queries();
    assert_eq!(mutations.len(), 1_000);
    for q in &mutations {
        compile(q, "postgres").unwrap_or_else(|e| panic!("mutation query failed: {e}"));
    }

    let mut group = c.benchmark_group("transpile");
    group.sample_size(200);
    group.measurement_time(Duration::from_secs(5));

    // Full corpus: average latency across all 1,000 queries (PRD target <0.5ms).
    group.bench_function("corpus_1000", |b| {
        b.iter(|| {
            for q in &queries {
                criterion::black_box(compile(criterion::black_box(q), "postgres").unwrap());
            }
        })
    });

    // Per-query worst-case: the longest/heaviest query in the corpus.
    let longest = queries
        .iter()
        .max_by_key(|q| q.len())
        .expect("corpus non-empty");
    group.bench_function(BenchmarkId::new("single_largest", longest.len()), |b| {
        b.iter(|| compile(criterion::black_box(longest), "postgres").unwrap())
    });

    // Mutation corpus: average latency across all 1,000 mutation statements
    // (PRD v2.0 target <25µs per mutation query).
    group.bench_function("mutation_1000", |b| {
        b.iter(|| {
            for q in &mutations {
                criterion::black_box(compile(criterion::black_box(q), "postgres").unwrap());
            }
        })
    });

    let largest_mutation = mutations
        .iter()
        .max_by_key(|q| q.len())
        .expect("mutation corpus non-empty");
    group.bench_function(
        BenchmarkId::new("single_largest_mutation", largest_mutation.len()),
        |b| b.iter(|| compile(criterion::black_box(largest_mutation), "postgres").unwrap()),
    );

    // Dialect sweep across all four backends.
    for dialect in ["postgres", "sqlite", "duckdb", "mysql"] {
        group.bench_function(format!("dialect_{dialect}_1000"), |b| {
            b.iter(|| {
                for q in &queries {
                    criterion::black_box(compile(criterion::black_box(q), dialect).unwrap());
                }
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_transpile);
criterion_main!(benches);
