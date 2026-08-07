// Package pipeql is a Go binding for PipeQL via the libpipeql C ABI (cgo).
//
// It compiles PipeQL source into target-dialect SQL with a fully isolated
// parameter map, giving Go applications the same injection-safe, polyglot
// query pipeline as the Rust core.
//
// Prerequisites:
//   - Build libpipeql once: `cargo build --release -p pipeql-cffi`
//   - Point CGO_LDFLAGS at the produced library, e.g. on Linux:
//       CGO_LDFLAGS="-L$PWD/target/release -lpipeql_cffi"
//
// Usage:
//
//	res, err := pipeql.Compile("from users | filter age >= $min | select [id]", "postgres")
//	if err != nil { log.Fatal(err) }
//	fmt.Println(res.SQL)
//	fmt.Println(res.Params) // ["min"]
package pipeql

/*
#cgo LDFLAGS: -lpipeql_cffi
#cgo CFLAGS: -I${SRCDIR}/../crates/pipeql-cffi/include
#include <stdlib.h>
#include "libpipeql.h"
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"unsafe"
)

// ErrKind classifies a compile failure, mirroring the C error kinds.
type ErrKind int

const (
	ErrNone    ErrKind = C.PIPEQL_ERR_NONE
	ErrParse   ErrKind = C.PIPEQL_ERR_PARSE
	ErrAnalysis ErrKind = C.PIPEQL_ERR_ANALYSIS
	ErrCodegen ErrKind = C.PIPEQL_ERR_CODEGEN
)

// Err is an error returned by Compile.
type Err struct {
	Kind    ErrKind
	Message string
}

func (e *Err) Error() string { return e.Message }

// Result is the outcome of a successful compile.
type Result struct {
	// SQL is the target-dialect SQL text with positional placeholders.
	SQL string
	// Params is the ordered list of extracted parameter names.
	Params []string
	// StatementType is "select", "insert", "update", "delete", or "create_table".
	StatementType string
	// IsMutation is true for insert/update/delete statements.
	IsMutation bool
	// Analysis is the full semantic analysis (param map, types, occurrences).
	Analysis json.RawMessage
}

// Compile a PipeQL source string for a target dialect
// ("postgres" default, "sqlite", "duckdb", "mysql").
func Compile(source, dialect string) (*Result, error) {
	if dialect == "" {
		dialect = "postgres"
	}
	csrc := C.CString(source)
	defer C.free(unsafe.Pointer(csrc))
	cdial := C.CString(dialect)
	defer C.free(unsafe.Pointer(cdial))

	var cerr C.PipeqlError
	res := C.pipeql_compile(csrc, cdial, &cerr)
	if res == nil {
		defer C.pipeql_error_clear(&cerr)
		msg := "PipeQL compile failed"
		if cerr.message != nil {
			msg = C.GoString(cerr.message)
		}
		return nil, &Err{Kind: ErrKind(cerr.kind), Message: msg}
	}
	defer C.pipeql_result_free(res)

	sql := C.GoString(res.sql)
	paramsJSON := C.GoString(res.params_json)
	statementType := C.GoString(res.statement_type)
	analysisJSON := C.GoString(res.analysis_json)

	var params []string
	if err := json.Unmarshal([]byte(paramsJSON), &params); err != nil {
		return nil, fmt.Errorf("pipeql: invalid params JSON: %w", err)
	}

	return &Result{
		SQL:           sql,
		Params:        params,
		StatementType: statementType,
		IsMutation:    res.is_mutation != 0,
		Analysis:      json.RawMessage(analysisJSON),
	}, nil
}

// MustCompile compiles or panics. Convenient for static/codegen contexts.
func MustCompile(source, dialect string) *Result {
	res, err := Compile(source, dialect)
	if err != nil {
		panic(err)
	}
	return res
}

// Version returns the PipeQL library version.
func Version() string {
	return C.GoString(C.pipeql_version())
}
