import assert from "node:assert/strict";
import { PipeQL } from "../src/builder.js";

// 1. Read pipeline source composition
{
  const q = PipeQL.from("notes")
    .filter("is_archived == 0")
    .sort(["created_at desc"])
    .take(10);
  assert.equal(
    q.source(),
    "from notes | filter is_archived == 0 | sort [created_at desc] | take 10",
  );
}

// 2. Read pipeline compiles through the WASM facade
{
  const q = PipeQL.from("notes")
    .filter("is_archived == 0")
    .sort(["created_at desc"])
    .take(10);
  const { sql, statementType } = await q.compile("postgres");
  assert.ok(sql.includes("SELECT * FROM notes"));
  assert.ok(sql.includes("WHERE (is_archived = 0)"));
  assert.ok(sql.includes("ORDER BY created_at DESC"));
  assert.ok(sql.includes("LIMIT 10"));
  assert.equal(statementType, "select");
}

// 3. Joins
{
  assert.equal(
    PipeQL.from("a").join("b", "a.id == b.a_id").source(),
    "from a | join b on a.id == b.a_id",
  );
  assert.equal(
    PipeQL.from("a").leftJoin("b", "a.id == b.a_id").source(),
    "from a | left join b on a.id == b.a_id",
  );
  assert.equal(
    PipeQL.from("a").fullJoin("b", "a.id == b.a_id").source(),
    "from a | full join b on a.id == b.a_id",
  );
}

// 4. Group + filter
{
  const q = PipeQL.from("orders")
    .group(["region"], "total = sum(amt), n = count(*)")
    .filter("total > 100");
  assert.equal(
    q.source(),
    "from orders | group [region] (total = sum(amt), n = count(*)) | filter total > 100",
  );
}

// 5. Union accepts builder and string
{
  const other = PipeQL.from("archived").select(["id"]);
  const q = PipeQL.from("active").select(["id"]).union(other);
  assert.equal(
    q.source(),
    "from active | select [id] | union from archived | select [id]",
  );
  const q2 = PipeQL.from("a").select(["id"]).union("from b | select [id]");
  assert.equal(q2.source(), "from a | select [id] | union from b | select [id]");
  const q3 = PipeQL.from("a").select(["id"]).unionAll("from b | select [id]");
  assert.equal(q3.source(), "from a | select [id] | union all from b | select [id]");
}

// 6. Object insert → auto params + values
{
  const q = PipeQL.into("notes").insert({ title: "Hi", flag: 1 });
  assert.equal(q.source(), "into notes | insert [title = $b0, flag = $b1]");
  assert.deepEqual(q.values, { b0: "Hi", b1: 1 });
  const { sql, params } = await q.compile("sqlite");
  assert.deepEqual(params, ["b0", "b1"]);
  assert.ok(sql.includes("INSERT INTO notes (title, flag) VALUES (?, ?)"));
}

// 7. Object insert keeps null distinct
{
  const q = PipeQL.into("t").insert({ a: null, b: 2 });
  assert.equal(q.source(), "into t | insert [a = $b0, b = $b1]");
  assert.deepEqual(q.values, { b0: null, b1: 2 });
}

// 8. Assignment strings pass through unchanged
{
  const q = PipeQL.into("t").insert(["title = $title", "flag = 1"]);
  assert.equal(q.source(), "into t | insert [title = $title, flag = 1]");
  assert.deepEqual(q.values, {});
}

// 9. Update + delete
{
  const q = PipeQL.from("notes").filter("id == $id").update({ title: "new" });
  assert.equal(q.source(), "from notes | filter id == $id | update [title = $b0]");
  const d = PipeQL.from("notes").filter("id == $id").delete();
  assert.equal(d.source(), "from notes | filter id == $id | delete");
}

// 9b. updateAll / deleteAll escape hatch
{
  const u = PipeQL.from("notes").updateAll({ title: "new" });
  assert.equal(u.source(), "from notes | update all [title = $b0]");
  const c = await u.compile("sqlite");
  assert.equal(c.sql, "UPDATE notes\nSET title = ?;");
  const d = PipeQL.from("notes").deleteAll();
  assert.equal(d.source(), "from notes | delete all");
  assert.equal((await d.compile("sqlite")).sql, "DELETE FROM notes;");
}

// 10. Upsert chain compiles
{
  const q = PipeQL.into("users")
    .upsert({ id: 1, name: "Ann" })
    .conflict(["id"])
    .doUpdate({ name: "Ann" });
  assert.equal(
    q.source(),
    "into users | upsert [id = $b0, name = $b1] | conflict [id] | do update [name = $b2]",
  );
  const { statementType, sql } = await q.compile("postgres");
  assert.equal(statementType, "upsert");
  assert.ok(sql.includes("ON CONFLICT (id) DO UPDATE SET name = $3"));
}

// 11. compile() carries builder values
{
  const { values } = await PipeQL.into("t").insert({ a: 1 }).compile("sqlite");
  assert.deepEqual(values, { b0: 1 });
}

// 12. String form
{
  assert.equal(String(PipeQL.from("t").take(1)), "from t | take 1");
}

console.log("all @pipeql/js builder smoke tests passed");
