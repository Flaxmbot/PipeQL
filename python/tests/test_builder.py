import os
import sqlite3
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

import pipeql_python as p
from pipeql_python.builder import PipeQL


def test_read_pipeline_source():
    q = (
        PipeQL.from_("notes")
        .filter("is_archived == 0")
        .sort(["created_at desc"])
        .take(10)
    )
    assert q.source() == "from notes | filter is_archived == 0 | sort [created_at desc] | take 10"


def test_read_pipeline_compiles():
    q = (
        PipeQL.from_("notes")
        .filter("is_archived == 0")
        .sort(["created_at desc"])
        .take(10)
    )
    result = q.compile("postgres")
    assert "SELECT * FROM notes" in result["sql"]
    assert "WHERE (is_archived = 0)" in result["sql"]
    assert "ORDER BY created_at DESC" in result["sql"]
    assert "LIMIT 10" in result["sql"]
    assert result["statement_type"] == "select"


def test_joins():
    assert PipeQL.from_("a").join("b", "a.id == b.a_id").source() == "from a | join b on a.id == b.a_id"
    assert (
        PipeQL.from_("a").left_join("b", "a.id == b.a_id").source()
        == "from a | left join b on a.id == b.a_id"
    )
    assert (
        PipeQL.from_("a").full_join("b", "a.id == b.a_id").source()
        == "from a | full join b on a.id == b.a_id"
    )


def test_select_and_group():
    q = (
        PipeQL.from_("orders")
        .group(["region"], "total = sum(amt), n = count(*)")
        .filter("total > 100")
    )
    assert (
        q.source()
        == "from orders | group [region] (total = sum(amt), n = count(*)) | filter total > 100"
    )
    assert PipeQL.from_("t").select("id, name").source() == "from t | select [id, name]"
    assert PipeQL.from_("t").select(["id", "name"]).source() == "from t | select [id, name]"


def test_union_accepts_builder_and_string():
    other = PipeQL.from_("archived").select(["id"])
    q = PipeQL.from_("active").select(["id"]).union(other)
    assert q.source() == "from active | select [id] | union from archived | select [id]"
    q2 = PipeQL.from_("a").select(["id"]).union("from b | select [id]")
    assert q2.source() == "from a | select [id] | union from b | select [id]"


def test_object_insert_generates_params_and_values():
    q = PipeQL.into_("notes").insert({"title": "Hi", "flag": 1})
    assert q.source() == "into notes | insert [title = $b0, flag = $b1]"
    assert q.values == {"b0": "Hi", "b1": 1}
    result = q.compile("sqlite")
    assert result["params"] == ["b0", "b1"]
    assert "INSERT INTO notes (title, flag) VALUES (?, ?)" in result["sql"]


def test_insert_accepts_assignment_strings():
    q = PipeQL.into_("t").insert(["title = $title", "flag = 1"])
    assert q.source() == "into t | insert [title = $title, flag = 1]"
    assert q.values == {}
    q2 = PipeQL.into_("t").insert("title = $title")
    assert q2.source() == "into t | insert [title = $title]"


def test_update_and_delete():
    q = PipeQL.from_("notes").filter("id == $id").update({"title": "new"})
    assert q.source() == "from notes | filter id == $id | update [title = $b0]"
    assert q.values == {"b0": "new"}
    d = PipeQL.from_("notes").filter("id == $id").delete()
    assert d.source() == "from notes | filter id == $id | delete"


def test_update_all_delete_all_escape_hatch():
    # `update all` / `delete all` bypass the filter guard (explicit opt-in).
    u = PipeQL.from_("notes").update_all({"title": "new"})
    assert u.source() == "from notes | update all [title = $b0]"
    compiled = u.compile("sqlite")
    assert compiled["sql"].startswith("UPDATE notes")
    assert "WHERE" not in compiled["sql"]
    d = PipeQL.from_("notes").delete_all()
    assert d.source() == "from notes | delete all"
    assert d.compile("sqlite")["sql"] == "DELETE FROM notes;"


def test_upsert_chain():
    q = (
        PipeQL.into_("users")
        .upsert({"id": 1, "name": "Ann"})
        .conflict(["id"])
        .do_update({"name": "Ann"})
    )
    assert (
        q.source()
        == "into users | upsert [id = $b0, name = $b1] | conflict [id] | do update [name = $b2]"
    )
    result = q.compile("postgres")
    assert result["statement_type"] == "upsert"
    assert "ON CONFLICT (id) DO UPDATE SET name = $3" in result["sql"]


def test_value_null_is_distinct():
    q = PipeQL.into_("t").insert({"a": None, "b": 2})
    assert q.source() == "into t | insert [a = $b0, b = $b1]"
    assert q.values == {"b0": None, "b1": 2}


def test_compile_merges_builder_values_and_params():
    q = PipeQL.into_("t").insert({"title": "x"}).compile("sqlite", other=5)
    assert q["values"] == {"b0": "x", "other": 5}


def test_str_repr():
    q = PipeQL.from_("t").take(1)
    assert str(q) == "from t | take 1"


# ---------------------------------------------------------------------------
# Driver integration: db.query(builder) duck-typing + value merging
# ---------------------------------------------------------------------------


def test_driver_accepts_builder():
    from pipeql_python.driver import create_pipeql_driver

    conn = sqlite3.connect(":memory:")
    db = create_pipeql_driver(conn)
    db.execute("table t [id int primary auto, title string, flag int default 0]", {})

    q = PipeQL.into_("t").insert({"title": "Hi", "flag": 1})
    r = db.query(q)
    assert r["last_id"] == 1
    assert r["changes"] == 1

    sel = PipeQL.from_("t").select(["id", "title", "flag"]).filter("id == $id")
    rows = db.query(sel, {"id": 1})
    assert rows == [{"id": 1, "title": "Hi", "flag": 1}]

    # builder with auto values + explicit params merge
    upd = PipeQL.from_("t").filter("id == $id").update({"title": "Bye"})
    r2 = db.query(upd, {"id": 1})
    assert r2["changes"] == 1
    assert db.query(PipeQL.from_("t").select(["title"])) == [{"title": "Bye"}]

    conn.close()


def test_driver_accepts_raw_string_still():
    from pipeql_python.driver import create_pipeql_driver

    conn = sqlite3.connect(":memory:")
    db = create_pipeql_driver(conn)
    db.execute("table t [id int primary auto, title string]", {})
    r = db.execute("into t | insert [title = $title]", {"title": "x"})
    assert r["last_id"] == 1
    conn.close()
