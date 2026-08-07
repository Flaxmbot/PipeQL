# PipeQL Examples

Each example shows the source query and the generated SQL for a target
dialect. Compile any example yourself:

```bash
pipeql compile -- "$(cat examples/engineering-salaries.pql)"
pipeql compile --dialect sqlite -- "$(cat examples/author-activity.pql)"
```

The `--` is required because every example begins with a `--` comment line.

## 1. engineering-salaries.pql → Postgres

```pipeql
from employees
| filter department == 'engineering' and salary >= $min_salary
| sort [salary desc]
| take 50
```

```sql
SELECT * FROM employees
WHERE ((department = $1) AND (salary >= $2))
ORDER BY salary DESC
LIMIT 50;
```

Parameters: `["engineering", "min_salary"]` — the string literal is extracted
too, so nothing user-controlled ever reaches the SQL text.

## 2. author-activity.pql → SQLite

```pipeql
from posts as p
| join users as u on p.author_id == u.id
| group [u.id, u.name] (post_count = count(p.id), avg_score = avg(p.score))
| filter post_count > $threshold
| sort [post_count desc]
```

```sql
SELECT u.id, u.name, COUNT(p.id) AS post_count, AVG(p.score) AS avg_score FROM posts AS p
INNER JOIN users AS u ON (p.author_id = u.id)
GROUP BY u.id, u.name
HAVING (count(p.id) > ?)
ORDER BY post_count DESC;
```

Parameters: `["threshold"]`.

## 3. hourly-events.pql → DuckDB

```pipeql
from events
| filter event_type in ['login', 'signup'] and occurred_at >= $since
| derive [hour = extract_hour(occurred_at)]
| group [hour] (cnt = count(*))
| select [hour, cnt]
```

```sql
SELECT extract_hour(occurred_at), COUNT(*) AS cnt FROM events
WHERE ((event_type IN (?, ?)) AND (occurred_at >= ?))
GROUP BY extract_hour(occurred_at);
```

Parameters: `["login", "signup", "since"]`.

## 4. paged-orders.pql → MySQL

```pipeql
from orders
| filter status != 'cancelled'
| select [order_id, customer_id, amount as total]
| skip 20
| take 25
```

```sql
SELECT order_id, customer_id, amount AS total FROM orders
WHERE (status <> ?)
LIMIT 25
OFFSET 20;
```

Parameters: `["cancelled"]`.

## 5. create-table.pql → SQLite (v2.0 DDL)

```pipeql
table notes [
  id int primary auto,
  title string not null,
  content string not null,
  category string default 'Personal',
  is_pinned int default 0,
  created_at timestamp default current_timestamp
]
```

```sql
CREATE TABLE IF NOT EXISTS notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  content TEXT NOT NULL,
  category TEXT DEFAULT 'Personal',
  is_pinned INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

Parameters: none — DDL defaults are schema metadata, inlined by the compiler.

## 6. insert-note.pql → Postgres (v2.0 insert)

```pipeql
into notes
| insert [
    title = $title,
    content = $content,
    category = 'Personal',
    is_pinned = 0
  ]
```

```sql
INSERT INTO notes (title, content, category, is_pinned) VALUES ($1, $2, $3, $4) RETURNING *;
```

Parameters: `["title", "content", "Personal", "0"]`. Postgres appends
`RETURNING *` so generated primary keys come back; question-style dialects
(SQLite/DuckDB/MySQL) omit it.

## 7. update-note.pql → Postgres (v2.0 update)

```pipeql
from notes
| filter id == $id and is_archived == 0
| update [
    title = $title,
    is_pinned = 1,
    updated_at = CURRENT_TIMESTAMP
  ]
```

```sql
UPDATE notes
SET title = $1, is_pinned = $2, updated_at = CURRENT_TIMESTAMP
WHERE ((id = $3) AND (is_archived = $4));
```

Parameters: `["title", "1", "id", "0"]` — `SET` values bind before `WHERE`
values. `CURRENT_TIMESTAMP` is inlined; every other assigned value is a
parameter.

## 8. delete-note.pql → Postgres (v2.0 delete)

```pipeql
from notes
| filter id == $id or is_archived == 1
| delete
```

```sql
DELETE FROM notes
WHERE ((id = $1) OR (is_archived = $2));
```

Parameters: `["id", "1"]`.
