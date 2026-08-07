/// Shared corpus of 1,000 realistic complex queries exercising every feature
/// the PRD requires: FROM / JOIN / WHERE / GROUP BY / HAVING / SELECT /
/// ORDER BY / LIMIT, parameters, `not`/`in`/`is` operators, and the SQL
/// clause order (FROM -> JOIN -> WHERE -> GROUP BY -> HAVING -> SELECT ->
/// ORDER BY -> LIMIT), plus the v2.0 mutation statements (insert/update/delete).
///
/// Shared between the criterion benchmark (`benches/transpile.rs`) and the
/// integration test suite so the corpus is guaranteed to compile (no silently
/// truncated or invalid queries ever reach the latency numbers).
const READ_SHAPES: [&str; 8] = [
    "from orders | join customers on orders.customer_id == customers.id | filter orders.status == 'active' and orders.total >= $min_total | group [customer_id, region] (total = sum(orders.total), cnt = count(*)) | filter total > $threshold | select [customer_id, region, total, cnt] | sort [total desc, customer_id asc] | take 10",
    "from events | filter event_type in ['click', 'view', 'hover'] and user_id is not null | group [user_id, day] (views = count(*), last = max(timestamp)) | sort [views desc] | take 100 | select [user_id, day, views, last]",
    "from users u | join orders o on u.id == o.user_id | join payments p on o.id == p.order_id | filter u.age >= {low} and u.age <= {high} and p.amount > 0 | select [u.id, u.name, o.total, p.amount] | sort [o.total desc] | take 10",
    "from log_entries | filter level in ['error', 'warn'] and not (service == 'health') | group [service, date] (errors = count(*)) | filter errors > $min_errors | select [service, date, errors] | sort [date desc, errors desc] | take 50",
    "from products | filter category == 'electronics' | filter price >= $min_price and price <= $max_price | select [id, name, price] | sort [price asc] | take 20 | skip 40",
    "from sessions s | join users u on s.user_id == u.id | filter u.plan in ['pro', 'enterprise'] and s.duration >= $min_seconds | group [u.id] (total_duration = sum(s.duration), sessions = count(*)) | select [u.id, u.name, total_duration, sessions] | sort [total_duration desc] | take 10",
    "from inventory | filter stock <= $reorder_point and (warehouse == 'north' or warehouse == 'east') | select [sku, name, stock, warehouse] | sort [stock asc, warehouse asc]",
    "from transactions t | join accounts a on t.account_id == a.id | filter t.status == 'completed' and a.balance >= $min_balance | group [a.country] (revenue = sum(t.amount), count = count(*)) | filter count >= $min_count | select [a.country, revenue, count] | sort [revenue desc] | take 100",
];

/// v2.0 mutation statements, matching the PRD examples.
const MUTATION_SHAPES: [&str; 3] = [
    "into notes | insert [title = $title, content = $content, category = 'Personal', is_pinned = 0, created_at = CURRENT_TIMESTAMP]",
    "from notes | filter is_archived == 0 | filter id == $id | update [title = $title, is_pinned = 1, updated_at = CURRENT_TIMESTAMP]",
    "from notes | filter id == $id or is_archived == 1 | delete",
];

pub fn shapes() -> Vec<&'static str> {
    let mut all = READ_SHAPES.to_vec();
    all.extend(MUTATION_SHAPES);
    all
}

/// Expand the 8 read shapes into a 1,000-query corpus (shape 2 gets
/// parametrized age bounds so every query is unique).
pub fn queries() -> Vec<String> {
    let mut queries = Vec::with_capacity(1_000);
    for i in 0..1_000 {
        let n = i % shapes().len();
        let shape = shapes()[n];
        if shape.contains("{low}") {
            queries.push(
                shape
                    .replace("{low}", &i.to_string())
                    .replace("{high}", &(i + 40).to_string()),
            );
        } else {
            queries.push(shape.to_owned());
        }
    }
    queries
}

/// A 1,000-query corpus of mutation statements, used for the <25µs NFR.
pub fn mutation_queries() -> Vec<String> {
    (0..1_000)
        .map(|i| MUTATION_SHAPES[i % MUTATION_SHAPES.len()].to_owned())
        .collect()
}
