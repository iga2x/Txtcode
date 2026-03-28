/// Task 29.3 — WASM Execution in Runtime
///
/// Allows Txtcode scripts to load and call exported functions from `.wasm`
/// binary modules.  Requires the `wasm-exec` feature.
///
/// # Functions
///
/// ```text
/// wasm_load(path: string) -> int          // load .wasm file, return handle
/// wasm_call(handle: int, fn: string, args: array) -> value
/// wasm_close(handle: int) -> null
/// ```
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

// ── Feature-gated implementation ─────────────────────────────────────────────

#[cfg(feature = "wasm-exec")]
mod wasm_impl {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use wasmtime::{Engine, Instance, Module, Store};

    struct WasmModule {
        engine: Engine,
        store: Store<()>,
        instance: Instance,
    }

    // Safety: WasmModule is only accessed behind Mutex.
    unsafe impl Send for WasmModule {}

    lazy_static::lazy_static! {
        static ref WASM_HANDLES: Mutex<HashMap<i64, WasmModule>> =
            Mutex::new(HashMap::new());
        static ref WASM_NEXT_ID: Mutex<i64> = Mutex::new(1);
    }

    pub fn wasm_load(args: &[Value]) -> Result<Value, RuntimeError> {
        let path = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(RuntimeError::new("wasm_load: expected string path".to_string())),
        };

        let engine = Engine::default();
        let module = Module::from_file(&engine, &path)
            .map_err(|e| RuntimeError::new(format!("wasm_load: failed to load '{}': {}", path, e)))?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| RuntimeError::new(format!("wasm_load: instantiation failed: {}", e)))?;

        let id = {
            let mut next = WASM_NEXT_ID.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };

        WASM_HANDLES.lock().unwrap().insert(id, WasmModule { engine, store, instance });
        Ok(Value::Integer(id))
    }

    pub fn wasm_call(args: &[Value]) -> Result<Value, RuntimeError> {
        let handle = match args.first() {
            Some(Value::Integer(n)) => *n,
            _ => return Err(RuntimeError::new("wasm_call: expected integer handle".to_string())),
        };
        let fn_name = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(RuntimeError::new("wasm_call: expected string function name".to_string())),
        };
        let call_args: Vec<wasmtime::Val> = match args.get(2) {
            Some(Value::Array(arr)) => arr
                .iter()
                .map(|v| match v {
                    Value::Integer(n) => wasmtime::Val::I64(*n),
                    Value::Float(f) => wasmtime::Val::F64((*f).to_bits()),
                    _ => wasmtime::Val::I64(0),
                })
                .collect(),
            None => vec![],
            _ => return Err(RuntimeError::new("wasm_call: args must be an array".to_string())),
        };

        let mut guard = WASM_HANDLES.lock().unwrap();
        let wm = guard.get_mut(&handle).ok_or_else(|| {
            RuntimeError::new(format!("wasm_call: invalid handle {}", handle))
        })?;

        let func = wm
            .instance
            .get_func(&mut wm.store, &fn_name)
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "wasm_call: function '{}' not found in module",
                    fn_name
                ))
            })?;

        let mut results = vec![wasmtime::Val::I64(0); func.ty(&wm.store).results().len()];
        func.call(&mut wm.store, &call_args, &mut results)
            .map_err(|e| RuntimeError::new(format!("wasm_call: execution error: {}", e)))?;

        match results.first() {
            Some(wasmtime::Val::I64(n)) => Ok(Value::Integer(*n)),
            Some(wasmtime::Val::I32(n)) => Ok(Value::Integer(*n as i64)),
            Some(wasmtime::Val::F64(bits)) => Ok(Value::Float(f64::from_bits(*bits))),
            Some(wasmtime::Val::F32(bits)) => Ok(Value::Float(f32::from_bits(*bits) as f64)),
            None => Ok(Value::Null),
            _ => Ok(Value::Null),
        }
    }

    pub fn wasm_close(args: &[Value]) -> Result<Value, RuntimeError> {
        let handle = match args.first() {
            Some(Value::Integer(n)) => *n,
            _ => return Err(RuntimeError::new("wasm_close: expected integer handle".to_string())),
        };
        WASM_HANDLES.lock().unwrap().remove(&handle);
        Ok(Value::Null)
    }
}

// ── Public dispatch ───────────────────────────────────────────────────────────

pub fn call_function(name: &str, _args: &[Value]) -> Result<Value, RuntimeError> {
    match name {
        "wasm_load" => {
            #[cfg(feature = "wasm-exec")]
            return wasm_impl::wasm_load(args);
            #[cfg(not(feature = "wasm-exec"))]
            return Err(RuntimeError::new(
                "wasm_load requires the 'wasm-exec' feature. \
                 Rebuild with: cargo build --features wasm-exec"
                    .to_string(),
            ));
        }
        "wasm_call" => {
            #[cfg(feature = "wasm-exec")]
            return wasm_impl::wasm_call(args);
            #[cfg(not(feature = "wasm-exec"))]
            return Err(RuntimeError::new(
                "wasm_call requires the 'wasm-exec' feature. \
                 Rebuild with: cargo build --features wasm-exec"
                    .to_string(),
            ));
        }
        "wasm_close" => {
            #[cfg(feature = "wasm-exec")]
            return wasm_impl::wasm_close(args);
            #[cfg(not(feature = "wasm-exec"))]
            return Err(RuntimeError::new(
                "wasm_close requires the 'wasm-exec' feature. \
                 Rebuild with: cargo build --features wasm-exec"
                    .to_string(),
            ));
        }
        _ => Err(RuntimeError::new(format!("wasm_exec: unknown function '{}'", name))),
    }
}
