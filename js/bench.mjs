/**
 * PipeQL JS/TS (WASM) Benchmark
 *
 * Measures compile latency for 1,000 read queries + 1,000 mutations
 * using the WASM-backed SDK. Includes FFI/WASM-call overhead.
 */
import { compile, initWasm } from "./src/index.js";

const READ_SHAPES = [
  "from orders | join customers on orders.customer_id == customers.id | filter orders.status == 'active' and orders.total >= $min_total | group [customer_id, region] (total = sum(orders.total), cnt = count(*)) | filter total > $threshold | select [customer_id, region, total, cnt] | sort [total desc, customer_id asc] | take 10",
  "from events | filter event_type in ['click', 'view', 'hover'] and user_id is not null | group [user_id, day] (views = count(*), last = max(timestamp)) | sort [views desc] | take 100 | select [user_id, day, views, last]",
  "from users u | join orders o on u.id == o.user_id | join payments p on o.id == p.order_id | filter u.age >= {low} and u.age <= {high} and p.amount > 0 | select [u.id, u.name, o.total, p.amount] | sort [o.total desc] | take 10",
  "from log_entries | filter level in ['error', 'warn'] and not (service == 'health') | group [service, date] (errors = count(*)) | filter errors > $min_errors | select [service, date, errors] | sort [date desc, errors desc] | take 50",
  "from products | filter category == 'electronics' | filter price >= $min_price and price <= $max_price | select [id, name, price] | sort [price asc] | take 20 | skip 40",
  "from sessions s | join users u on s.user_id == u.id | filter u.plan in ['pro', 'enterprise'] and s.duration >= $min_seconds | group [u.id] (total_duration = sum(s.duration), sessions = count(*)) | select [u.id, u.name, total_duration, sessions] | sort [total_duration desc] | take 10",
  "from inventory | filter stock <= $reorder_point and (warehouse == 'north' or warehouse == 'east') | select [sku, name, stock, warehouse] | sort [stock asc, warehouse asc]",
  "from transactions t | join accounts a on t.account_id == a.id | filter t.status == 'completed' and a.balance >= $min_balance | group [a.country] (revenue = sum(t.amount), count = count(*)) | filter count >= $min_count | select [a.country, revenue, count] | sort [revenue desc] | take 100",
];

const MUTATION_SHAPES = [
  "into notes | insert [title = $title, content = $content, category = 'Personal', is_pinned = 0, created_at = CURRENT_TIMESTAMP]",
  "from notes | filter is_archived == 0 | filter id == $id | update [title = $title, is_pinned = 1, updated_at = CURRENT_TIMESTAMP]",
  "from notes | filter id == $id or is_archived == 1 | delete",
];

function buildCorpus(n) {
  const all = [...READ_SHAPES, ...MUTATION_SHAPES];
  const queries = [];
  for (let i = 0; i < n; i++) {
    const shape = all[i % all.length];
    if (shape.includes("{low}")) {
      queries.push(shape.replace("{low}", String(i)).replace("{high}", String(i + 40)));
    } else {
      queries.push(shape);
    }
  }
  return queries;
}

function buildMutationCorpus(n) {
  const queries = [];
  for (let i = 0; i < n; i++) {
    queries.push(MUTATION_SHAPES[i % MUTATION_SHAPES.length]);
  }
  return queries;
}

async function benchmarkCompile(queries, label) {
  // Warmup: compile first query once
  await compile(queries[0], "postgres");

  const times = [];
  for (const q of queries) {
    const start = performance.now();
    await compile(q, "postgres");
    times.push(performance.now() - start);
  }

  times.sort((a, b) => a - b);
  const total = times.reduce((a, b) => a + b, 0);
  const mean = total / times.length;
  const median = times[Math.floor(times.length / 2)];
  const p95 = times[Math.floor(times.length * 0.95)];
  const p99 = times[Math.floor(times.length * 0.99)];
  const min = times[0];
  const max = times[times.length - 1];

  console.log(`\n=== ${label} (${queries.length} queries) ===`);
  console.log(`  Total:   ${total.toFixed(2)} ms`);
  console.log(`  Mean:    ${mean.toFixed(3)} ms/query`);
  console.log(`  Median:  ${median.toFixed(3)} ms/query`);
  console.log(`  Min:     ${min.toFixed(3)} ms`);
  console.log(`  Max:     ${max.toFixed(3)} ms`);
  console.log(`  P95:     ${p95.toFixed(3)} ms`);
  console.log(`  P99:     ${p99.toFixed(3)} ms`);

  return { total, mean, median, p95, p99, min, max };
}

async function main() {
  console.log("PipeQL JS/TS (WASM) Benchmark");
  console.log("=".repeat(40));

  await initWasm();

  const readQueries = buildCorpus(1000);
  const mutationQueries = buildMutationCorpus(1000);

  const readResult = await benchmarkCompile(readQueries, "Read Corpus (1000 queries)");
  const mutationResult = await benchmarkCompile(mutationQueries, "Mutation Corpus (1000 queries)");

  // Single largest query
  const largest = READ_SHAPES[0];
  const start = performance.now();
  await compile(largest, "postgres");
  const singleTime = performance.now() - start;
  console.log(`\n=== Single Largest Query ===`);
  console.log(`  Time: ${singleTime.toFixed(3)} ms`);

  // Per-query average
  console.log(`\n=== Summary ===`);
  console.log(`  Read avg:     ${(readResult.total / 1000).toFixed(3)} ms/query`);
  console.log(`  Mutation avg: ${(mutationResult.total / 1000).toFixed(3)} ms/query`);
}

main().catch(console.error);
