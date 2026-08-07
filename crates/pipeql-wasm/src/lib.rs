//! WASM bindings for PipeQL, powering the `@pipeql/js` JavaScript SDK.
//!
//! Compile with:
//! ```sh
//! wasm-pack build crates/pipeql-wasm --target web --out-dir ../../js/dist
//! ```

use wasm_bindgen::prelude::*;

use pipeql_core::{api, PipeQLError, StatementType};

/// A compiled query, exposed to JavaScript as a plain object.
#[derive(Debug)]
#[wasm_bindgen]
pub struct Compiled {
    sql: String,
    params: js_sys::Array,
    statement_type: StatementType,
    is_mutation: bool,
    analysis: JsValue,
}

#[wasm_bindgen]
impl Compiled {
    /// The target-dialect SQL text.
    #[wasm_bindgen(getter)]
    pub fn sql(&self) -> String {
        self.sql.clone()
    }

    /// The ordered array of parameter names.
    #[wasm_bindgen(getter)]
    pub fn params(&self) -> js_sys::Array {
        self.params.clone()
    }

    /// The statement kind: "select", "insert", "update", "delete",
    /// "create_table".
    #[wasm_bindgen(getter)]
    pub fn statement_type(&self) -> String {
        self.statement_type.as_str().to_string()
    }

    /// True for mutations (insert/update/delete).
    #[wasm_bindgen(getter)]
    pub fn is_mutation(&self) -> bool {
        self.is_mutation
    }

    /// The full analysis document (param map, inferred types, occurrences).
    #[wasm_bindgen(getter)]
    pub fn analysis(&self) -> JsValue {
        self.analysis.clone()
    }
}

/// Build a `Compiled` from a core result.
fn to_compiled(c: pipeql_core::CompiledQuery) -> Compiled {
    Compiled {
        sql: c.sql,
        params: c
            .params
            .iter()
            .map(|s| JsValue::from_str(s))
            .collect::<js_sys::Array>(),
        statement_type: c.statement_type,
        is_mutation: c.is_mutation,
        analysis: serde_wasm_bindgen::to_value(&c.analysis).unwrap_or(JsValue::NULL),
    }
}

/// Compile a PipeQL source string for a target dialect.
///
/// `dialect` defaults to `"postgres"`. Returns a `Compiled` object with `sql`,
/// `params`, `statementType`, `isMutation`, and `analysis` properties, or
/// throws on error.
#[wasm_bindgen]
pub fn compile(source: &str, dialect: Option<String>) -> Result<Compiled, JsValue> {
    let dialect = dialect.as_deref().unwrap_or("postgres");
    api::compile(source, dialect)
        .map(to_compiled)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compile a PipeQL source string, validating columns against a JSON schema
/// catalog. The catalog format matches the Rust `Catalog` type:
///
/// ```json
/// { "tables": { "users": { "name": "users", "columns": [ { "name": "id", "ty": "Integer" } ] } } }
/// ```
#[wasm_bindgen(js_name = compileWithCatalog)]
pub fn compile_with_catalog(
    source: &str,
    dialect: Option<String>,
    catalog_json: &str,
) -> Result<Compiled, JsValue> {
    let dialect = dialect.as_deref().unwrap_or("postgres");
    let catalog: pipeql_core::Catalog = serde_json::from_str(catalog_json)
        .map_err(|e| JsValue::from_str(&format!("invalid catalog JSON: {e}")))?;
    api::compile_with_catalog(source, dialect, Some(&catalog))
        .map(to_compiled)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse-only: returns a JSON description of the lossless AST (spans, comments,
/// steps). Useful for editors and tooling.
#[wasm_bindgen(js_name = parseAst)]
pub fn parse_ast(source: &str) -> Result<JsValue, JsValue> {
    match api::parse_statement(source) {
        Ok(stmt) => serde_wasm_bindgen::to_value(&stmt)
            .map_err(|e| JsValue::from_str(&format!("serialization error: {e}"))),
        Err(PipeQLError::Parse(errs)) => {
            let message = errs
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            Err(JsValue::from_str(&message))
        }
        Err(_) => Err(JsValue::from_str("unexpected error")),
    }
}

/// List of supported target dialects.
#[wasm_bindgen(js_name = supportedDialects)]
pub fn supported_dialects() -> js_sys::Array {
    api::supported_dialects()
        .iter()
        .map(|s| JsValue::from_str(s))
        .collect::<js_sys::Array>()
}

/// PipeQL version string.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn compile_smoke() {
        let c = compile("from users | filter age > $min | select [id]", None).unwrap();
        assert!(c.sql().contains("SELECT id FROM users"));
        assert_eq!(c.params().length(), 1);
        assert_eq!(c.statement_type(), "select");
        assert!(!c.is_mutation());
    }

    #[test]
    fn compile_mutation_metadata() {
        let c = compile("into notes | insert [title = $t]", None).unwrap();
        assert_eq!(c.statement_type(), "insert");
        assert!(c.is_mutation());
    }

    #[test]
    fn compile_errors_are_js_values() {
        let err = compile("from users | explode", None).unwrap_err();
        assert!(err.is_string());
    }
}
