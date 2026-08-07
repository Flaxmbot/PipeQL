package pipeql

import (
	"strings"
	"testing"
)

func TestCompilePostgres(t *testing.T) {
	res, err := Compile(
		"from users | filter age >= $min and status == 'active' | select [id, name] | sort [name asc] | take 10",
		"postgres",
	)
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if !strings.Contains(res.SQL, "SELECT id, name FROM users") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
	if !strings.Contains(res.SQL, "$1") {
		t.Errorf("expected postgres placeholder in %s", res.SQL)
	}
	if len(res.Params) != 2 {
		t.Errorf("expected 2 params, got %v", res.Params)
	}
}

func TestCompileSQLiteUsesQuestionMarks(t *testing.T) {
	res, err := Compile("from t | filter id == $id | take 5", "sqlite")
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if !strings.Contains(res.SQL, "?") {
		t.Errorf("expected ? placeholder in %s", res.SQL)
	}
}

func TestCompileError(t *testing.T) {
	_, err := Compile("from users | explode", "postgres")
	if err == nil {
		t.Fatal("expected error")
	}
	perr, ok := err.(*Err)
	if !ok {
		t.Fatalf("expected *Err, got %T", err)
	}
	if perr.Kind != ErrParse {
		t.Errorf("expected ErrParse, got %d", perr.Kind)
	}
	if !strings.Contains(perr.Message, "explode") {
		t.Errorf("message should mention bad step: %s", perr.Message)
	}
}

func TestMustCompile(t *testing.T) {
	res := MustCompile("from t | select [*]", "postgres")
	if !strings.Contains(res.SQL, "SELECT * FROM t") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
}

func TestStatementMetadata(t *testing.T) {
	sel, err := Compile("from users | filter id == $id | select [id]", "postgres")
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if sel.StatementType != "select" {
		t.Errorf("expected statement_type 'select', got %q", sel.StatementType)
	}
	if sel.IsMutation {
		t.Error("select must not be a mutation")
	}

	ins, err := Compile("into notes | insert [title = $t]", "sqlite")
	if err != nil {
		t.Fatalf("compile insert: %v", err)
	}
	if ins.StatementType != "insert" {
		t.Errorf("expected statement_type 'insert', got %q", ins.StatementType)
	}
	if !ins.IsMutation {
		t.Error("insert must be a mutation")
	}

	upd, err := Compile("from notes | filter id == $id | update [is_pinned = 1]", "sqlite")
	if err != nil {
		t.Fatalf("compile update: %v", err)
	}
	if upd.StatementType != "update" {
		t.Errorf("expected statement_type 'update', got %q", upd.StatementType)
	}

	del, err := Compile("from notes | filter id == $id | delete", "sqlite")
	if err != nil {
		t.Fatalf("compile delete: %v", err)
	}
	if del.StatementType != "delete" {
		t.Errorf("expected statement_type 'delete', got %q", del.StatementType)
	}

	ddl, err := Compile("table notes [id int primary auto]", "sqlite")
	if err != nil {
		t.Fatalf("compile ddl: %v", err)
	}
	if ddl.StatementType != "create_table" {
		t.Errorf("expected statement_type 'create_table', got %q", ddl.StatementType)
	}
	if ddl.IsMutation {
		t.Error("create_table must not be a mutation")
	}
}

func TestVersion(t *testing.T) {
	if Version() == "" {
		t.Error("empty version")
	}
}

func TestStatementMetadataUpsert(t *testing.T) {
	res, err := Compile(
		"into users | upsert [name = $name, email = $email] | conflict [email] | do update [name = $name]",
		"postgres",
	)
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if res.StatementType != "upsert" {
		t.Errorf("expected statement_type 'upsert', got %q", res.StatementType)
	}
	if !res.IsMutation {
		t.Error("upsert must be a mutation")
	}
	if !strings.Contains(res.SQL, "ON CONFLICT (email) DO UPDATE SET name = $1") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
}

func TestStatementMetadataUnion(t *testing.T) {
	res, err := Compile(
		"from active_users | select [id, name] | union from archived_users | select [id, name]",
		"postgres",
	)
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if res.StatementType != "union" {
		t.Errorf("expected statement_type 'union', got %q", res.StatementType)
	}
	if res.IsMutation {
		t.Error("union must not be a mutation")
	}
	if !strings.Contains(res.SQL, "UNION") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
}

func TestCompileUnionAll(t *testing.T) {
	res, err := Compile(
		"from active_users | select [id] | union all from archived_users | select [id]",
		"postgres",
	)
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if !strings.Contains(res.SQL, "UNION ALL") {
		t.Errorf("expected UNION ALL in sql: %s", res.SQL)
	}
}

func TestCompileSubquery(t *testing.T) {
	res, err := Compile(
		"from orders | filter customer_id in (from customers | filter region == 'EU' | select [id])",
		"postgres",
	)
	if err != nil {
		t.Fatalf("compile: %v", err)
	}
	if !strings.Contains(res.SQL, "IN (SELECT id FROM customers") {
		t.Errorf("unexpected sql: %s", res.SQL)
	}
	if len(res.Params) != 1 || res.Params[0] != "EU" {
		t.Errorf("expected params [EU], got %v", res.Params)
	}
}
