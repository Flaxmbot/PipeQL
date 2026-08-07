//! C-FFI bindings for PipeQL, exposed as `libpipeql`.
//!
//! These symbols form a stable C ABI consumed by C/C++, Go (via cgo), and any
//! other FFI-capable language. Memory ownership: every returned pointer that
//! must be freed is documented as "owned"; release it with the matching
//! `pipeql_*_free` function.

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pipeql_core::{api, PipeQLError};

/// Error kinds, mirroring `PipeQLError` variants. Stable across versions.
pub const PIPEQL_ERR_NONE: i32 = 0;
pub const PIPEQL_ERR_PARSE: i32 = 1;
pub const PIPEQL_ERR_ANALYSIS: i32 = 2;
pub const PIPEQL_ERR_CODEGEN: i32 = 3;

/// Result of a successful compile. All fields are NUL-terminated owned
/// strings; free with `pipeql_result_free`.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PipeqlResult {
    /// The target-dialect SQL.
    pub sql: *mut c_char,
    /// JSON array of parameter names, e.g. `["min_age","top"]`.
    pub params_json: *mut c_char,
    /// The statement kind: "select", "insert", "update", "delete",
    /// "create_table", "upsert", "union".
    pub statement_type: *mut c_char,
    /// Non-zero when the statement is a mutation (insert/update/delete/upsert).
    pub is_mutation: i32,
    /// JSON document of the full analysis (param map, types, occurrences).
    pub analysis_json: *mut c_char,
    /// Number of extracted parameters.
    pub parameter_count: i32,
}

/// Error payload. `message` is owned; free with `pipeql_error_clear`.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PipeqlError {
    /// One of `PIPEQL_ERR_*`.
    pub kind: i32,
    /// NUL-terminated owned message.
    pub message: *mut c_char,
}

/// Compile a PipeQL source string into target-dialect SQL.
///
/// # Safety
///
/// `source` and `dialect` must be valid NUL-terminated C strings for the
/// duration of the call. `err` must point to a `PipeqlError` zero-initialized
/// by the caller.
///
/// Returns a heap-allocated `PipeqlResult` on success (free with
/// `pipeql_result_free`), or `NULL` on failure with `err` populated.
#[no_mangle]
pub unsafe extern "C" fn pipeql_compile(
    source: *const c_char,
    dialect: *const c_char,
    err: *mut PipeqlError,
) -> *mut PipeqlResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let src = read_cstr(source);
        let dialect = read_cstr(dialect);
        match src {
            Some(s) => match api::compile(&s, dialect.as_deref().unwrap_or("postgres")) {
                Ok(compiled) => {
                    let params_json = serde_json::to_string(&compiled.params)
                        .unwrap_or_else(|_| "[]".to_string());
                    let analysis_json = serde_json::to_string(&compiled.analysis)
                        .unwrap_or_else(|_| "{}".to_string());
                    let parameter_count = compiled.params.len() as i32;
                    Box::into_raw(Box::new(PipeqlResult {
                        sql: into_cstring(compiled.sql),
                        params_json: into_cstring(params_json),
                        statement_type: into_cstring(compiled.statement_type.as_str().to_string()),
                        is_mutation: compiled.is_mutation as i32,
                        analysis_json: into_cstring(analysis_json),
                        parameter_count,
                    }))
                }
                Err(e) => {
                    write_error(err, classify(&e), &format!("{e}"));
                    ptr::null_mut()
                }
            },
            None => {
                write_error(err, PIPEQL_ERR_PARSE, "source must be a non-null string");
                ptr::null_mut()
            }
        }
    }));
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            write_error(err, PIPEQL_ERR_PARSE, "PipeQL panicked while compiling");
            ptr::null_mut()
        }
    }
}

/// Returns the PipeQL version as a static, NUL-terminated string. Never needs
/// freeing.
#[no_mangle]
pub extern "C" fn pipeql_version() -> *const c_char {
    let version = concat!(env!("CARGO_PKG_VERSION"), "\0");
    version.as_ptr() as *const c_char
}

/// Compile a PipeQL source string, optionally validating columns against a
/// JSON schema catalog.
///
/// # Safety
///
/// `source` and `dialect` must be valid NUL-terminated C strings.
/// `catalog_json` may be NULL (no validation) or a NUL-terminated JSON string
/// in the format: `{"tables":{"users":{"name":"users","columns":[{"name":"id",
/// "ty":"Integer"}]}}}`.
/// `err` must point to a `PipeqlError` zero-initialized by the caller.
///
/// Returns a heap-allocated `PipeqlResult` on success (free with
/// `pipeql_result_free`), or `NULL` on failure with `err` populated.
#[no_mangle]
pub unsafe extern "C" fn pipeql_compile_with_catalog(
    source: *const c_char,
    dialect: *const c_char,
    catalog_json: *const c_char,
    err: *mut PipeqlError,
) -> *mut PipeqlResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let src = read_cstr(source);
        let dialect = read_cstr(dialect);
        let catalog_str = read_cstr(catalog_json);

        match src {
            Some(s) => {
                let catalog = match catalog_str.as_deref() {
                    Some(js) => match serde_json::from_str::<pipeql_core::Catalog>(js) {
                        Ok(c) => Some(c),
                        Err(e) => {
                            write_error(
                                err,
                                PIPEQL_ERR_PARSE,
                                &format!("invalid catalog JSON: {e}"),
                            );
                            return ptr::null_mut();
                        }
                    },
                    None => None,
                };
                let catalog_ref = catalog.as_ref();
                match api::compile_with_catalog(
                    &s,
                    dialect.as_deref().unwrap_or("postgres"),
                    catalog_ref,
                ) {
                    Ok(compiled) => {
                        let params_json = serde_json::to_string(&compiled.params)
                            .unwrap_or_else(|_| "[]".to_string());
                        let analysis_json = serde_json::to_string(&compiled.analysis)
                            .unwrap_or_else(|_| "{}".to_string());
                        let parameter_count = compiled.params.len() as i32;
                        Box::into_raw(Box::new(PipeqlResult {
                            sql: into_cstring(compiled.sql),
                            params_json: into_cstring(params_json),
                            statement_type: into_cstring(
                                compiled.statement_type.as_str().to_string(),
                            ),
                            is_mutation: compiled.is_mutation as i32,
                            analysis_json: into_cstring(analysis_json),
                            parameter_count,
                        }))
                    }
                    Err(e) => {
                        write_error(err, classify(&e), &format!("{e}"));
                        ptr::null_mut()
                    }
                }
            }
            None => {
                write_error(err, PIPEQL_ERR_PARSE, "source must be a non-null string");
                ptr::null_mut()
            }
        }
    }));
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            write_error(err, PIPEQL_ERR_PARSE, "PipeQL panicked while compiling");
            ptr::null_mut()
        }
    }
}

/// Parse a PipeQL source into a lossless statement AST, returned as a JSON
/// string. Covers read pipelines, inserts, upserts, unions, and DDL.
///
/// The returned string must be freed with `pipeql_string_free`.
///
/// # Safety
///
/// `source` must be a valid NUL-terminated C string. `err` must point to a
/// `PipeqlError` zero-initialized by the caller.
///
/// Returns a heap-allocated JSON string on success (free with
/// `pipeql_string_free`), or `NULL` on failure with `err` populated.
#[no_mangle]
pub unsafe extern "C" fn pipeql_parse(source: *const c_char, err: *mut PipeqlError) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let src = read_cstr(source);
        match src {
            Some(s) => match api::parse_statement(&s) {
                Ok(stmt) => match serde_json::to_string(&stmt) {
                    Ok(json) => into_cstring(json),
                    Err(e) => {
                        write_error(
                            err,
                            PIPEQL_ERR_CODEGEN,
                            &format!("AST serialization failed: {e}"),
                        );
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    write_error(err, classify(&e), &format!("{e}"));
                    ptr::null_mut()
                }
            },
            None => {
                write_error(err, PIPEQL_ERR_PARSE, "source must be a non-null string");
                ptr::null_mut()
            }
        }
    }));
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            write_error(err, PIPEQL_ERR_PARSE, "PipeQL panicked while parsing");
            ptr::null_mut()
        }
    }
}

/// Return the list of supported dialect names as a JSON array string,
/// e.g. `["postgres","sqlite","duckdb","mysql"]`.
///
/// The returned string must be freed with `pipeql_string_free`.
#[no_mangle]
pub extern "C" fn pipeql_supported_dialects() -> *mut c_char {
    let dialects = api::supported_dialects();
    let json = serde_json::to_string(&dialects).unwrap_or_else(|_| "[]".to_string());
    into_cstring(json)
}

/// Free a string previously returned by `pipeql_parse` or
/// `pipeql_supported_dialects`.
///
/// # Safety
///
/// `s` must be a pointer returned by `pipeql_parse` or
/// `pipeql_supported_dialects`, or NULL.
#[no_mangle]
pub unsafe extern "C" fn pipeql_string_free(s: *mut c_char) {
    free_cstring(s);
}

/// Free a result previously returned by `pipeql_compile`.
///
/// # Safety
///
/// `res` must be a pointer from `pipeql_compile` or NULL.
#[no_mangle]
pub unsafe extern "C" fn pipeql_result_free(res: *mut PipeqlResult) {
    if res.is_null() {
        return;
    }
    let boxed = Box::from_raw(res);
    free_cstring(boxed.sql);
    free_cstring(boxed.params_json);
    free_cstring(boxed.statement_type);
    free_cstring(boxed.analysis_json);
}

/// Free a message previously written into a `PipeqlError`.
///
/// # Safety
///
/// `err` must point to a `PipeqlError` populated by PipeQL.
#[no_mangle]
pub unsafe extern "C" fn pipeql_error_clear(err: *mut PipeqlError) {
    if err.is_null() {
        return;
    }
    let e = &mut *err;
    if !e.message.is_null() {
        drop(CString::from_raw(e.message));
        e.message = ptr::null_mut();
    }
    e.kind = PIPEQL_ERR_NONE;
}

fn classify(e: &PipeQLError) -> i32 {
    match e {
        PipeQLError::Parse(_) => PIPEQL_ERR_PARSE,
        PipeQLError::Analysis(_) => PIPEQL_ERR_ANALYSIS,
        PipeQLError::Codegen(_) => PIPEQL_ERR_CODEGEN,
    }
}

unsafe fn read_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string())
}

fn into_cstring(s: String) -> *mut c_char {
    CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| {
        CString::new("<unprintable>")
            .expect("static string is NUL-free")
            .into_raw()
    })
}

unsafe fn free_cstring(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

unsafe fn write_error(err: *mut PipeqlError, kind: i32, message: &str) {
    if err.is_null() {
        return;
    }
    let e = &mut *err;
    if !e.message.is_null() {
        drop(CString::from_raw(e.message));
    }
    e.kind = kind;
    e.message = into_cstring(message.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    unsafe fn cstr(s: &str) -> *const c_char {
        CString::new(s).unwrap().into_raw() as *const c_char
    }

    unsafe fn free_cstr(ptr: *const c_char) {
        if !ptr.is_null() {
            drop(CString::from_raw(ptr as *mut c_char));
        }
    }

    #[test]
    fn test_compile_basic() {
        unsafe {
            let src = cstr("from users | filter age >= $min | select [id, name]");
            let dialect = cstr("postgres");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile(src, dialect, &mut err);
            assert!(!res.is_null(), "compile should succeed");
            let r = &*res;
            assert!(CStr::from_ptr(r.sql)
                .to_str()
                .unwrap()
                .contains("SELECT id, name FROM users"));
            assert_eq!(CStr::from_ptr(r.statement_type).to_str().unwrap(), "select");
            assert_eq!(r.is_mutation, 0);
            assert_eq!(r.parameter_count, 1);
            pipeql_result_free(res);
            free_cstr(src);
            free_cstr(dialect);
        }
    }

    #[test]
    fn test_compile_upsert_returns_upsert_type() {
        unsafe {
            let src =
                cstr("into users | upsert [name = $n] | conflict [email] | do update [name = $n]");
            let dialect = cstr("postgres");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile(src, dialect, &mut err);
            assert!(!res.is_null(), "compile upsert should succeed");
            let r = &*res;
            assert_eq!(CStr::from_ptr(r.statement_type).to_str().unwrap(), "upsert");
            assert_eq!(r.is_mutation, 1);
            pipeql_result_free(res);
            free_cstr(src);
            free_cstr(dialect);
        }
    }

    #[test]
    fn test_compile_union_returns_union_type() {
        unsafe {
            let src = cstr("from a | select [id] | union all from b | select [id]");
            let dialect = cstr("postgres");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile(src, dialect, &mut err);
            assert!(!res.is_null(), "compile union should succeed");
            let r = &*res;
            assert_eq!(CStr::from_ptr(r.statement_type).to_str().unwrap(), "union");
            assert_eq!(r.is_mutation, 0);
            pipeql_result_free(res);
            free_cstr(src);
            free_cstr(dialect);
        }
    }

    #[test]
    fn test_compile_with_catalog_valid() {
        unsafe {
            let src = cstr("from users | select [id, name]");
            let dialect = cstr("postgres");
            let catalog = cstr(
                r#"{"tables":{"users":{"name":"users","columns":[{"name":"id","ty":"Integer"},{"name":"name","ty":"String"}]}}}"#,
            );
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile_with_catalog(src, dialect, catalog, &mut err);
            if res.is_null() {
                let msg = if err.message.is_null() {
                    "<null>".to_string()
                } else {
                    CStr::from_ptr(err.message).to_str().unwrap().to_string()
                };
                panic!(
                    "compile with valid catalog failed: kind={}, msg={}",
                    err.kind, msg
                );
            }
            pipeql_result_free(res);
            free_cstr(src);
            free_cstr(dialect);
            free_cstr(catalog);
        }
    }

    #[test]
    fn test_compile_with_catalog_invalid_column() {
        unsafe {
            let src = cstr("from users | select [nope]");
            let dialect = cstr("postgres");
            let catalog = cstr(
                r#"{"tables":{"users":{"name":"users","columns":[{"name":"id","ty":"Integer"}]}}}"#,
            );
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile_with_catalog(src, dialect, catalog, &mut err);
            assert!(res.is_null(), "compile with invalid column should fail");
            let msg = if err.message.is_null() {
                "<null>".to_string()
            } else {
                CStr::from_ptr(err.message).to_str().unwrap().to_string()
            };
            assert_eq!(
                err.kind, PIPEQL_ERR_ANALYSIS,
                "expected ANALYSIS error, got kind={}, msg={}",
                err.kind, msg
            );
            assert!(msg.contains("nope"), "error should mention column: {msg}");
            pipeql_error_clear(&mut err);
            free_cstr(src);
            free_cstr(dialect);
            free_cstr(catalog);
        }
    }

    #[test]
    fn test_compile_with_catalog_null() {
        unsafe {
            let src = cstr("from users | select [id]");
            let dialect = cstr("sqlite");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile_with_catalog(src, dialect, ptr::null(), &mut err);
            assert!(!res.is_null(), "null catalog should behave like compile");
            pipeql_result_free(res);
            free_cstr(src);
            free_cstr(dialect);
        }
    }

    #[test]
    fn test_parse_returns_json() {
        unsafe {
            let src = cstr("from users | filter id == $id | select [id]");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let json_ptr = pipeql_parse(src, &mut err);
            assert!(!json_ptr.is_null(), "parse should succeed");
            let json_str = CStr::from_ptr(json_ptr).to_str().unwrap();
            let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
            assert!(
                parsed.is_object() || parsed.is_array(),
                "should be valid JSON"
            );
            pipeql_string_free(json_ptr);
            free_cstr(src);
        }
    }

    #[test]
    fn test_parse_error() {
        unsafe {
            let src = cstr("from users | explode");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let json_ptr = pipeql_parse(src, &mut err);
            assert!(json_ptr.is_null(), "parse of bad source should fail");
            assert_eq!(err.kind, PIPEQL_ERR_PARSE);
            pipeql_error_clear(&mut err);
            free_cstr(src);
        }
    }

    #[test]
    fn test_supported_dialects() {
        unsafe {
            let json_ptr = pipeql_supported_dialects();
            assert!(!json_ptr.is_null());
            let json_str = CStr::from_ptr(json_ptr).to_str().unwrap();
            let dialects: Vec<String> = serde_json::from_str(json_str).unwrap();
            assert_eq!(dialects.len(), 4);
            assert!(dialects.contains(&"postgres".to_string()));
            assert!(dialects.contains(&"sqlite".to_string()));
            assert!(dialects.contains(&"duckdb".to_string()));
            assert!(dialects.contains(&"mysql".to_string()));
            pipeql_string_free(json_ptr);
        }
    }

    #[test]
    fn test_parameter_count() {
        unsafe {
            let src = cstr("from users | filter age >= $min and status == $s | select [id]");
            let dialect = cstr("postgres");
            let mut err = PipeqlError {
                kind: PIPEQL_ERR_NONE,
                message: ptr::null_mut(),
            };
            let res = pipeql_compile(src, dialect, &mut err);
            assert!(!res.is_null());
            let r = &*res;
            assert_eq!(r.parameter_count, 2);
            let params: Vec<String> =
                serde_json::from_str(CStr::from_ptr(r.params_json).to_str().unwrap()).unwrap();
            assert_eq!(r.parameter_count, params.len() as i32);
            pipeql_result_free(res);
            free_cstr(src);
            free_cstr(dialect);
        }
    }

    #[test]
    fn test_compile_all_dialects() {
        for dialect in &["postgres", "sqlite", "duckdb", "mysql"] {
            unsafe {
                let src = cstr("from users | filter id == $id | take 5");
                let d = cstr(dialect);
                let mut err = PipeqlError {
                    kind: PIPEQL_ERR_NONE,
                    message: ptr::null_mut(),
                };
                let res = pipeql_compile(src, d, &mut err);
                assert!(
                    !res.is_null(),
                    "compile should succeed for dialect: {dialect}"
                );
                pipeql_result_free(res);
                free_cstr(src);
                free_cstr(d);
            }
        }
    }
}
