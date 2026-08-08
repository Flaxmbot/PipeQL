package pipeql

import (
	"strings"
	"testing"
)

func TestBuilderReadPipeline(t *testing.T) {
	q := From("notes").
		Filter("is_archived == 0").
		Sort([]string{"created_at desc"}).
		Take(10)
	want := "from notes | filter is_archived == 0 | sort [created_at desc] | take 10"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}

	res, err := q.Compile("postgres")
	if err != nil {
		t.Fatalf("Compile: %v", err)
	}
	if !strings.Contains(res.SQL, "SELECT * FROM notes") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
	if !strings.Contains(res.SQL, "ORDER BY created_at DESC") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
}

func TestBuilderJoins(t *testing.T) {
	cases := []struct {
		name string
		got  string
		want string
	}{
		{"inner", From("a").Join("b", "a.id == b.a_id").Source(), "from a | join b on a.id == b.a_id"},
		{"left", From("a").LeftJoin("b", "a.id == b.a_id").Source(), "from a | left join b on a.id == b.a_id"},
		{"full", From("a").FullJoin("b", "a.id == b.a_id").Source(), "from a | full join b on a.id == b.a_id"},
	}
	for _, c := range cases {
		if c.got != c.want {
			t.Errorf("%s: Source() = %q, want %q", c.name, c.got, c.want)
		}
	}
}

func TestBuilderGroup(t *testing.T) {
	q := From("orders").Group([]string{"region"}, "total = sum(amt), n = count(*)").Filter("total > 100")
	want := "from orders | group [region] (total = sum(amt), n = count(*)) | filter total > 100"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}
}

func TestBuilderUnion(t *testing.T) {
	other := From("archived").Select([]string{"id"})
	q := From("active").Select([]string{"id"}).Union(other)
	want := "from active | select [id] | union from archived | select [id]"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}
	q2 := From("a").Select([]string{"id"}).UnionAll("from b | select [id]")
	if got := q2.Source(); got != "from a | select [id] | union all from b | select [id]" {
		t.Errorf("Source() = %q", got)
	}
}

func TestBuilderObjectInsert(t *testing.T) {
	// Maps are sorted for deterministic SQL: flag < title.
	q := Into("notes").Insert(map[string]any{"title": "Hi", "flag": 1})
	want := "into notes | insert [flag = $b0, title = $b1]"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}
	vals := q.Values()
	if vals["b0"] != 1 || vals["b1"] != "Hi" {
		t.Errorf("Values() = %v", vals)
	}

	res, err := q.Compile("sqlite")
	if err != nil {
		t.Fatalf("Compile: %v", err)
	}
	if !strings.Contains(res.SQL, "INSERT INTO notes (flag, title) VALUES (?, ?)") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
	if len(res.Params) != 2 {
		t.Errorf("expected 2 params, got %v", res.Params)
	}
}

func TestBuilderObjectInsertOrderedPairs(t *testing.T) {
	// Pairs preserves insertion order exactly.
	q := Into("notes").Insert(PairsOf("title", "Hi", "flag", 1))
	want := "into notes | insert [title = $b0, flag = $b1]"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}
}

func TestBuilderNullDistinct(t *testing.T) {
	q := Into("t").Insert(map[string]any{"a": nil, "b": 2})
	want := "into t | insert [a = $b0, b = $b1]"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}
	if q.Values()["b0"] != nil {
		t.Errorf("expected nil value, got %v", q.Values()["b0"])
	}
}

func TestBuilderAssignmentStrings(t *testing.T) {
	q := Into("t").Insert([]string{"title = $title", "flag = 1"})
	if got := q.Source(); got != "into t | insert [title = $title, flag = 1]" {
		t.Errorf("Source() = %q", got)
	}
	if len(q.Values()) != 0 {
		t.Errorf("expected no values, got %v", q.Values())
	}
}

func TestBuilderUpdateDelete(t *testing.T) {
	q := From("notes").Filter("id == $id").Update(map[string]any{"title": "new"})
	if got := q.Source(); got != "from notes | filter id == $id | update [title = $b0]" {
		t.Errorf("Source() = %q", got)
	}
	d := From("notes").Filter("id == $id").Delete()
	if got := d.Source(); got != "from notes | filter id == $id | delete" {
		t.Errorf("Source() = %q", got)
	}
}

func TestBuilderUpdateAllDeleteAllEscapeHatch(t *testing.T) {
	u := From("notes").UpdateAll(map[string]any{"title": "new"})
	if got := u.Source(); got != "from notes | update all [title = $b0]" {
		t.Errorf("Source() = %q", got)
	}
	res, err := u.Compile("sqlite")
	if err != nil || res == nil || res.SQL == "" || strings.Contains(res.SQL, "WHERE") {
		t.Errorf("UpdateAll compiled err=%v res=%v, want full-table UPDATE without WHERE", err, res)
	}
	d := From("notes").DeleteAll()
	if got := d.Source(); got != "from notes | delete all" {
		t.Errorf("Source() = %q", got)
	}
	res, err = d.Compile("sqlite")
	if err != nil || res == nil || res.SQL != "DELETE FROM notes;" {
		t.Errorf("DeleteAll compiled err=%v res=%v", err, res)
	}
}

func TestBuilderUpsert(t *testing.T) {
	q := Into("users").
		Upsert(PairsOf("id", 1, "name", "Ann")).
		Conflict([]string{"id"}).
		DoUpdate(map[string]any{"name": "Ann"})
	want := "into users | upsert [id = $b0, name = $b1] | conflict [id] | do update [name = $b2]"
	if got := q.Source(); got != want {
		t.Errorf("Source() = %q, want %q", got, want)
	}
	res, err := q.Compile("postgres")
	if err != nil {
		t.Fatalf("Compile: %v", err)
	}
	if res.StatementType != "upsert" {
		t.Errorf("expected upsert, got %s", res.StatementType)
	}
	if !strings.Contains(res.SQL, "ON CONFLICT (id) DO UPDATE SET name = $3") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
}

func TestBuilderString(t *testing.T) {
	q := From("t").Take(1)
	if got := q.String(); got != "from t | take 1" {
		t.Errorf("String() = %q", got)
	}
}
