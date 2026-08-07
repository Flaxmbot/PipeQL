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
    /// "create_table".
    pub statement_type: *mut c_char,
    /// Non-zero when the statement is a mutation (insert/update/delete).
    pub is_mutation: i32,
    /// JSON document of the full analysis (param map, types, occurrences).
    pub analysis_json: *mut c_char,
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
                    Box::into_raw(Box::new(PipeqlResult {
                        sql: into_cstring(compiled.sql),
                        params_json: into_cstring(params_json),
                        statement_type: into_cstring(compiled.statement_type.as_str().to_string()),
                        is_mutation: compiled.is_mutation as i32,
                        analysis_json: into_cstring(analysis_json),
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
