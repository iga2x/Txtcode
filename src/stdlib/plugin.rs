/// Task L.3 — Native Plugin System
///
/// Provides a higher-level plugin API on top of the FFI layer.
///
/// # Plugin ABI (L.3 JSON protocol)
///
/// A Txtcode plugin is a shared library (`.so`/`.dylib`) that exports:
///
/// ```c
/// // Null-terminated plugin name
/// const char *txtcode_plugin_name(void);
///
/// // Null-terminated array of null-terminated function name strings (optional)
/// const char **txtcode_functions(void);
///
/// // Per-exported-function entry point:
/// //   args_json — JSON-encoded array of arguments
/// //   returns   — heap-allocated JSON-encoded result (caller frees via txtcode_free_result)
/// char *<fn_name>(const char *args_json);
///
/// // Release memory returned by any function
/// void txtcode_free_result(char *result);
/// ```
///
/// # Functions exposed to Txtcode scripts
/// - `plugin_load(path)` → int   (handle; use with plugin_call)
/// - `plugin_functions(path)` → array[string]
/// - `plugin_call(handle, fn_name, args)` → value  (args = array; result decoded from JSON)
///
/// # Permission
/// Requires `sys.ffi` permission + path in the FFI allowlist (`--allow-ffi`).
///
/// # Feature gate
/// Requires `--features ffi`.
use crate::runtime::{RuntimeError, Value};
#[cfg(feature = "ffi")]
use crate::runtime::permissions::PermissionResource;
use crate::stdlib::PermissionChecker;

#[cfg(feature = "ffi")]
use std::sync::atomic::{AtomicI64, Ordering};
#[cfg(feature = "ffi")]
use std::sync::Mutex;
#[cfg(feature = "ffi")]
use std::collections::HashMap;

#[cfg(feature = "ffi")]
static PLUGIN_HANDLE_COUNTER: AtomicI64 = AtomicI64::new(1);

/// Map from integer handle → library path (used by plugin_call to look up lib).
#[cfg(feature = "ffi")]
static PLUGIN_HANDLE_MAP: Mutex<Option<HashMap<i64, String>>> = Mutex::new(None);

#[cfg(feature = "ffi")]
fn handle_map_get_path(handle: i64) -> Option<String> {
    let guard = PLUGIN_HANDLE_MAP.lock().ok()?;
    guard.as_ref()?.get(&handle).cloned()
}

#[cfg(feature = "ffi")]
fn handle_map_insert(handle: i64, path: String) {
    if let Ok(mut guard) = PLUGIN_HANDLE_MAP.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(handle, path);
    }
}

pub struct PluginLib;

impl PluginLib {
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        match name {
            "plugin_load" => Self::plugin_load(args, permission_checker),
            "plugin_functions" => Self::plugin_functions(args, permission_checker),
            "plugin_call" => Self::plugin_call(args, permission_checker),
            _ => Err(RuntimeError::new(format!("Unknown plugin function: {}", name))),
        }
    }

    #[cfg(feature = "ffi")]
    fn require_ffi_permission(
        path: &str,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> Result<(), RuntimeError> {
        if let Some(checker) = permission_checker {
            checker.check_permission(
                &PermissionResource::System(format!("ffi:{}", path)),
                None,
            )?;
        }
        Ok(())
    }

    // ── plugin_load ──────────────────────────────────────────────────────────

    #[cfg(not(feature = "ffi"))]
    fn plugin_load(_args: &[Value], _pc: Option<&dyn PermissionChecker>) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "plugin_load requires the 'ffi' feature. Rebuild with: cargo build --features ffi".to_string(),
        ))
    }

    #[cfg(feature = "ffi")]
    fn plugin_load(args: &[Value], permission_checker: Option<&dyn PermissionChecker>) -> Result<Value, RuntimeError> {
        let path = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(RuntimeError::new("plugin_load(path): 'path' must be a string".to_string())),
        };
        Self::require_ffi_permission(&path, permission_checker)?;

        // Safety: we trust the path is in the FFI allowlist (enforced by permission system)
        let lib = unsafe {
            libloading::Library::new(&path)
                .map_err(|e| RuntimeError::new(format!("plugin_load: could not load '{}': {}", path, e)))?
        };

        // Call txtcode_plugin_name() to verify the library is a valid Txtcode plugin.
        unsafe {
            let sym: libloading::Symbol<unsafe extern "C" fn() -> *const std::os::raw::c_char> = lib
                .get(b"txtcode_plugin_name\0")
                .map_err(|e| RuntimeError::new(format!(
                    "plugin_load: missing symbol 'txtcode_plugin_name' in '{}': {}", path, e
                )))?;
            let ptr = sym();
            if ptr.is_null() {
                return Err(RuntimeError::new(format!(
                    "plugin_load: 'txtcode_plugin_name' returned NULL in '{}'", path
                )));
            }
        };

        // Store in the FFI library registry (keeps the library alive) and the
        // handle map so plugin_call can look it up by integer handle.
        {
            let mut registry = crate::stdlib::ffi::PLUGIN_LIBRARY_REGISTRY
                .lock()
                .map_err(|_| RuntimeError::new("plugin_load: internal lock error".to_string()))?;
            registry.insert(path.clone(), lib);
        }

        let handle = PLUGIN_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        handle_map_insert(handle, path);

        Ok(Value::Integer(handle))
    }

    // ── plugin_functions ─────────────────────────────────────────────────────

    #[cfg(not(feature = "ffi"))]
    fn plugin_functions(_args: &[Value], _pc: Option<&dyn PermissionChecker>) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "plugin_functions requires the 'ffi' feature. Rebuild with: cargo build --features ffi".to_string(),
        ))
    }

    #[cfg(feature = "ffi")]
    fn plugin_functions(args: &[Value], permission_checker: Option<&dyn PermissionChecker>) -> Result<Value, RuntimeError> {
        let path = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(RuntimeError::new("plugin_functions(path): 'path' must be a string".to_string())),
        };
        Self::require_ffi_permission(&path, permission_checker)?;

        let registry = crate::stdlib::ffi::PLUGIN_LIBRARY_REGISTRY
            .lock()
            .map_err(|_| RuntimeError::new("plugin_functions: internal lock error".to_string()))?;
        let lib = registry.get(&path).ok_or_else(|| {
            RuntimeError::new(format!("plugin_functions: plugin '{}' not loaded — call plugin_load() first", path))
        })?;

        let names: Vec<Value> = unsafe {
            let sym: libloading::Symbol<unsafe extern "C" fn() -> *const *const std::os::raw::c_char> = lib
                .get(b"txtcode_functions\0")
                .map_err(|e| RuntimeError::new(format!("plugin_functions: missing 'txtcode_functions' in '{}': {}", path, e)))?;
            let ptr = sym();
            if ptr.is_null() {
                return Ok(Value::Array(vec![]));
            }
            let mut i = 0;
            let mut out = Vec::new();
            loop {
                let fn_ptr = *ptr.add(i);
                if fn_ptr.is_null() { break; }
                let fn_name = std::ffi::CStr::from_ptr(fn_ptr).to_string_lossy().into_owned();
                out.push(Value::String(fn_name));
                i += 1;
            }
            out
        };

        Ok(Value::Array(names))
    }

    // ── plugin_call ──────────────────────────────────────────────────────────
    //
    // ABI (L.3): each exported function is a C symbol named after the function:
    //   char *fn_name(const char *args_json);
    //   void txtcode_free_result(char *result);
    //
    // `args_json` is a JSON-encoded array of the arguments.
    // The return value is a heap-allocated JSON string; call txtcode_free_result to free.

    #[cfg(not(feature = "ffi"))]
    fn plugin_call(_args: &[Value], _pc: Option<&dyn PermissionChecker>) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "plugin_call requires the 'ffi' feature. Rebuild with: cargo build --features ffi".to_string(),
        ))
    }

    #[cfg(feature = "ffi")]
    fn plugin_call(args: &[Value], permission_checker: Option<&dyn PermissionChecker>) -> Result<Value, RuntimeError> {
        if args.len() < 2 {
            return Err(RuntimeError::new(
                "plugin_call requires at least 2 arguments (handle, fn_name[, args])".to_string(),
            ));
        }
        let handle = match &args[0] {
            Value::Integer(n) => *n,
            _ => return Err(RuntimeError::new("plugin_call: first argument must be an integer handle from plugin_load".to_string())),
        };
        let fn_name = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(RuntimeError::new("plugin_call: second argument must be a string (function name)".to_string())),
        };
        let call_args: Vec<serde_json::Value> = match args.get(2) {
            Some(Value::Array(arr)) => arr.iter().map(Self::value_to_json).collect(),
            None | Some(Value::Null) => vec![],
            Some(other) => vec![Self::value_to_json(other)],
        };
        let args_json = serde_json::to_string(&call_args)
            .map_err(|e| RuntimeError::new(format!("plugin_call: failed to encode args as JSON: {}", e)))?;

        // Look up path from handle
        let path = handle_map_get_path(handle).ok_or_else(|| {
            RuntimeError::new(format!("plugin_call: unknown handle {} — call plugin_load() first", handle))
        })?;
        Self::require_ffi_permission(&path, permission_checker)?;

        let registry = crate::stdlib::ffi::PLUGIN_LIBRARY_REGISTRY
            .lock()
            .map_err(|_| RuntimeError::new("plugin_call: internal lock error".to_string()))?;
        let lib = registry.get(&path).ok_or_else(|| {
            RuntimeError::new(format!("plugin_call: plugin '{}' not loaded", path))
        })?;

        let result_json = unsafe {
            // Find the per-function symbol: `char *fn_name(const char *args_json)`
            let sym_name = format!("{}\0", fn_name);
            let sym: libloading::Symbol<unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::os::raw::c_char> =
                lib.get(sym_name.as_bytes())
                    .map_err(|e| RuntimeError::new(format!("plugin_call: symbol '{}' not found in '{}': {}", fn_name, path, e)))?;

            let c_args = std::ffi::CString::new(args_json.as_str())
                .map_err(|_| RuntimeError::new("plugin_call: args_json contains null byte".to_string()))?;

            let result_ptr = sym(c_args.as_ptr());
            if result_ptr.is_null() {
                return Ok(Value::Null);
            }

            // Copy result before freeing
            let result_str = std::ffi::CStr::from_ptr(result_ptr)
                .to_string_lossy()
                .into_owned();

            // Free the result via txtcode_free_result if the symbol exists
            if let Ok(free_sym) = lib.get::<unsafe extern "C" fn(*mut std::os::raw::c_char)>(b"txtcode_free_result\0") {
                free_sym(result_ptr);
            }

            result_str
        };

        // Decode JSON result to Value
        let json_val: serde_json::Value = serde_json::from_str(&result_json)
            .map_err(|e| RuntimeError::new(format!("plugin_call: result is not valid JSON: {} (got: {})", e, result_json)))?;
        Ok(Self::json_to_value(json_val))
    }

    // ── JSON ↔ Value helpers ─────────────────────────────────────────────────

    #[cfg(feature = "ffi")]
    fn value_to_json(v: &Value) -> serde_json::Value {
        match v {
            Value::Integer(n) => serde_json::Value::Number((*n).into()),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Boolean(b) => serde_json::Value::Bool(*b),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Null => serde_json::Value::Null,
            Value::Array(arr) => serde_json::Value::Array(arr.iter().map(Self::value_to_json).collect()),
            Value::Map(m) => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    m.iter().map(|(k, v)| (k.clone(), Self::value_to_json(v))).collect();
                serde_json::Value::Object(obj)
            }
            other => serde_json::Value::String(other.to_string()),
        }
    }

    #[cfg(feature = "ffi")]
    fn json_to_value(j: serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else {
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(Self::json_to_value).collect()),
            serde_json::Value::Object(obj) => {
                let mut map = indexmap::IndexMap::new();
                for (k, v) in obj {
                    map.insert(k, Self::json_to_value(v));
                }
                Value::Map(map)
            }
        }
    }
}
