import assert from "node:assert/strict";
import {
  compile,
  compileWithCatalog,
  parse,
  pipeql,
  supportedDialects,
  version,
} from "../src/index.js";

await compile("from t", "postgres");

// 1. Basic compile
{
  const { sql, params, parameterCount } = await compile(
    "from users | filter age >= $min_age and status == 'active' | select [id, name] | sort [name asc] | take 10",
    "postgres",
  );
  assert.ok(sql.includes("SELECT id, name FROM users"));
  assert.ok(sql.includes("age >= $1"));
  // String literals become bind params too (PRD: 100% parameter extraction).
  assert.deepEqual(params, ["min_age", "active"]);
  assert.equal(parameterCount, 2);
}

// 2. Dialects
{
  const sqlite = await compile("from users | filter id == $id | take 5", "sqlite");
  assert.ok(sqlite.sql.includes("LIMIT 5"));
  assert.ok(sqlite.sql.includes("id = ?"));
  const mysql = await compile("from users | filter id == $id | take 5", "mysql");
  assert.ok(mysql.sql.includes("LIMIT 5"));
}

// 2b. Statement metadata drives driver dispatch (no SQL prefix sniffing)
{
  const select = await compile("from users | filter id == $id | select [id]", "postgres");
  assert.equal(select.statementType, "select");
  assert.equal(select.isMutation, false);

  const insert = await compile("into notes | insert [title = $t, is_pinned = 0]", "sqlite");
  assert.equal(insert.statementType, "insert");
  assert.equal(insert.isMutation, true);

  const update = await compile("from notes | filter id == $id | update [is_pinned = 1]", "sqlite");
  assert.equal(update.statementType, "update");
  assert.equal(update.isMutation, true);

  const del = await compile("from notes | filter id == $id | delete", "sqlite");
  assert.equal(del.statementType, "delete");
  assert.equal(del.isMutation, true);

  const ddl = await compile("table notes [id int primary auto]", "sqlite");
  assert.equal(ddl.statementType, "create_table");
  assert.equal(ddl.isMutation, false);

  const upsert = await compile(
    "into users | upsert [name = $name, email = $email] | conflict [email] | do update [name = $name]",
    "postgres",
  );
  assert.equal(upsert.statementType, "upsert");
  assert.equal(upsert.isMutation, true);
  assert.ok(upsert.sql.includes("ON CONFLICT (email) DO UPDATE SET name = $3"));

  const union = await compile(
    "from active_users | select [id, name] | union from archived_users | select [id, name]",
    "postgres",
  );
  assert.equal(union.statementType, "union");
  assert.equal(union.isMutation, false);
  assert.ok(union.sql.includes("UNION"));
  assert.ok(!union.sql.includes("UNION ALL"));

  const unionAll = await compile(
    "from active_users | select [id, name] | union all from archived_users | select [id, name]",
    "postgres",
  );
  assert.ok(unionAll.sql.includes("UNION ALL"));
}

// 3. Tagged template with interpolation -> parameters, never inline
{
  const q = pipeql`from users | filter age >= ${18} and plan == ${"pro"} | select [id]`;
  const { sql, params, values } = await q.compile("postgres");
  assert.ok(sql.includes("age >= $1"));
  assert.ok(sql.includes("plan = $2"));
  assert.deepEqual(params, ["p0", "p1"]);
  assert.deepEqual(values, [18, "pro"]);
}

// 3b. Subquery (IN subquery)
{
  const result = await compile(
    "from orders | filter customer_id in (from customers | filter region == 'EU' | select [id])",
    "postgres",
  );
  assert.ok(result.sql.includes("IN (SELECT id FROM customers"));
  assert.ok(result.sql.includes("WHERE (region = $1)"));
  assert.deepEqual(result.params, ["EU"]);
}

// 4. Catalog validation catches unknown columns
{
  const catalog = {
    users: { name: "users", columns: [{ name: "id", ty: "Integer" }] },
  };
  const ok = await compileWithCatalog("from users | select [id]", catalog);
  assert.ok(ok.sql.includes("SELECT id FROM users"));
  await assert.rejects(
    () => compileWithCatalog("from users | select [nope]", catalog),
    /nope|Unknown column/,
  );
}

// 5. Errors carry actionable hints
{
  await assert.rejects(() => compile("from users | filter", "postgres"), /hint|expected/i);
}

// 6. AST parse
{
  const ast = await parse("from users | filter id == $x | select [id]");
  assert.equal(ast.source.name.name, "users");
  assert.ok(Array.isArray(ast.steps));
}

// 7. Introspection
{
  const dialects = await supportedDialects();
  assert.ok(dialects.includes("postgres"));
  assert.match(await version(), /^\d+\.\d+\.\d+$/);
}

console.log("all @pipeql/js smoke tests passed");
