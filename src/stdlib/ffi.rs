/// FFI stdlib — dynamic shared-library loading and calling.
///
/// # Feature gate
/// Full functionality requires the `ffi` Cargo feature:
/// ```text
/// cargo build --features ffi
/// ```
/// Without the feature the module still compiles but all functions return an
/// error explaining how to enable it.
///
/// # Functions
/// - `ffi_load(path: string) -> int`  — load a shared library, return a handle
/// - `ffi_call(handle: int, fn_name: string, ret_type: string, args: array) -> Value`
///   — call a C function; supported return types: `"int"`, `"float"`, `"void"`
/// - `ffi_close(handle: int) -> null` — unload a library
///
/// # Permission
/// All three functions require `sys.ffi` permission.
///
/// # Argument types (ffi_call args array)
/// Each element is coerced to i64:
/// - `int`   → passed directly as i64
/// - `float` → bit-cast (f64 bits) as i64
/// - `bool`  → 1 or 0
/// - `null`  → 0
///
/// Up to 4 arguments are supported in this version.
use crate::runtime::{RuntimeError, Value};

#[cfg(feature = "ffi")]
use libloading::{Library, Symbol};
#[cfg(feature = "ffi")]
use std::collections::HashMap;
#[cfg(feature = "ffi")]
use std::sync::Mutex;

#[cfg(feature = "ffi")]
lazy_static::lazy_static! {
    /// Global handle → Library map.  Libraries live until `ffi_close` is called.
    static ref LIBRARY_REGISTRY: Mutex<HashMap<i64, Library>> = Mutex::new(HashMap::new());
    /// Monotonically increasing handle counter.
    static ref NEXT_HANDLE: Mutex<i64> = Mutex::new(1);
}

pub struct FfiLib;

impl FfiLib {
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "ffi_load" => Self::ffi_load(args),
            "ffi_call" => Self::ffi_call(args),
            "ffi_close" => Self::ffi_close(args),
            _ => Err(RuntimeError::new(format!("Unknown FFI function: {}", name))),
        }
    }

    // ── ffi_load ──────────────────────────────────────────────────────────────

    fn ffi_load(args: &[Value]) -> Result<Value, RuntimeError> {
        #[cfg(not(feature = "ffi"))]
        return Err(RuntimeError::new(
            "ffi_load requires the 'ffi' feature. Rebuild with: cargo build --features ffi"
                .to_string(),
        ));

        #[cfg(feature = "ffi")]
        {
            let path = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(RuntimeError::new(
                        "ffi_load(path): 'path' must be a string".to_string(),
                    ))
                }
            };

            // SAFETY: loading a shared library is inherently unsafe. The caller
            // must have been granted sys.ffi permission before reaching this point.
            let lib = unsafe {
                Library::new(&path).map_err(|e| {
                    RuntimeError::new(format!("ffi_load: could not load '{}': {}", path, e))
                })?
            };

            let mut registry = LIBRARY_REGISTRY.lock().unwrap();
            let mut next = NEXT_HANDLE.lock().unwrap();
            let handle = *next;
            *next += 1;
            registry.insert(handle, lib);
            Ok(Value::Integer(handle))
        }
    }

    // ── ffi_call ──────────────────────────────────────────────────────────────

    fn ffi_call(args: &[Value]) -> Result<Value, RuntimeError> {
        #[cfg(not(feature = "ffi"))]
        return Err(RuntimeError::new(
            "ffi_call requires the 'ffi' feature. Rebuild with: cargo build --features ffi"
                .to_string(),
        ));

        #[cfg(feature = "ffi")]
        {
            let handle = match args.first() {
                Some(Value::Integer(h)) => *h,
                _ => {
                    return Err(RuntimeError::new(
                        "ffi_call(handle, fn_name, ret_type, args): 'handle' must be an int"
                            .to_string(),
                    ))
                }
            };

            let fn_name = match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(RuntimeError::new(
                        "ffi_call: 'fn_name' (arg 2) must be a string".to_string(),
                    ))
                }
            };

            let ret_type = match args.get(2) {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(RuntimeError::new(
                        "ffi_call: 'ret_type' (arg 3) must be a string (\"int\", \"float\", \"void\")".to_string(),
                    ))
                }
            };

            // Collect call arguments as i64 register values.
            let call_args: Vec<i64> = match args.get(3) {
                None | Some(Value::Null) => vec![],
                Some(Value::Array(arr)) => arr.iter().map(value_to_ffi_int).collect(),
                _ => {
                    return Err(RuntimeError::new(
                        "ffi_call: 'args' (arg 4) must be an array or null".to_string(),
                    ))
                }
            };

            if call_args.len() > 4 {
                return Err(RuntimeError::new(
                    "ffi_call: at most 4 arguments are supported in this version".to_string(),
                ));
            }

            let registry = LIBRARY_REGISTRY.lock().unwrap();
            let lib = registry.get(&handle).ok_or_else(|| {
                RuntimeError::new(format!(
                    "ffi_call: invalid handle {}. Did you call ffi_load first?",
                    handle
                ))
            })?;

            // SAFETY: we trust the caller to provide the correct function signature
            // via ret_type. Mismatches invoke undefined behaviour; the sys.ffi
            // permission gate restricts who can reach this call.
            unsafe { dispatch_call(lib, &fn_name, &ret_type, &call_args) }
        }
    }

    // ── ffi_close ─────────────────────────────────────────────────────────────

    fn ffi_close(args: &[Value]) -> Result<Value, RuntimeError> {
        #[cfg(not(feature = "ffi"))]
        return Err(RuntimeError::new(
            "ffi_close requires the 'ffi' feature. Rebuild with: cargo build --features ffi"
                .to_string(),
        ));

        #[cfg(feature = "ffi")]
        {
            let handle = match args.first() {
                Some(Value::Integer(h)) => *h,
                _ => {
                    return Err(RuntimeError::new(
                        "ffi_close(handle): 'handle' must be an int".to_string(),
                    ))
                }
            };

            let mut registry = LIBRARY_REGISTRY.lock().unwrap();
            // Dropping the Library value calls dlclose / FreeLibrary.
            registry.remove(&handle);
            Ok(Value::Null)
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Coerce a Value to an i64 for passing in a C integer register.
fn value_to_ffi_int(v: &Value) -> i64 {
    match v {
        Value::Integer(i) => *i,
        Value::Boolean(b) => i64::from(*b),
        // Bit-cast f64 → i64 so the float bits land in the integer register.
        Value::Float(f) => f.to_bits() as i64,
        Value::Null => 0,
        _ => 0,
    }
}

/// Dispatch a C function call with 0–4 i64 arguments and the given return type.
///
/// # Safety
/// The function pointer retrieved from `lib` must actually have the signature
/// implied by `ret_type` and the length of `call_args`.
#[cfg(feature = "ffi")]
unsafe fn dispatch_call(
    lib: &Library,
    fn_name: &str,
    ret_type: &str,
    call_args: &[i64],
) -> Result<Value, RuntimeError> {
    // Append a null terminator for libloading's symbol lookup.
    let sym_name = format!("{}\0", fn_name);
    let sym_bytes = sym_name.as_bytes();

    // Load the symbol as the most general "0-arg → i64" type and then
    // transmute the raw function pointer to the required concrete signature.
    type RawFn = unsafe extern "C" fn() -> i64;

    let sym: Symbol<RawFn> = lib.get(sym_bytes).map_err(|e| {
        RuntimeError::new(format!(
            "ffi_call: symbol '{}' not found in library: {}",
            fn_name, e
        ))
    })?;

    let raw: RawFn = *sym;

    match (call_args.len(), ret_type) {
        // 0 args
        (0, "void") => {
            let f: unsafe extern "C" fn() = std::mem::transmute(raw);
            f();
            Ok(Value::Null)
        }
        (0, "int") => Ok(Value::Integer(raw())),
        (0, "float") => {
            let f: unsafe extern "C" fn() -> f64 = std::mem::transmute(raw);
            Ok(Value::Float(f()))
        }

        // 1 arg
        (1, "void") => {
            let f: unsafe extern "C" fn(i64) = std::mem::transmute(raw);
            f(call_args[0]);
            Ok(Value::Null)
        }
        (1, "int") => {
            let f: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(raw);
            Ok(Value::Integer(f(call_args[0])))
        }
        (1, "float") => {
            let f: unsafe extern "C" fn(i64) -> f64 = std::mem::transmute(raw);
            Ok(Value::Float(f(call_args[0])))
        }

        // 2 args
        (2, "void") => {
            let f: unsafe extern "C" fn(i64, i64) = std::mem::transmute(raw);
            f(call_args[0], call_args[1]);
            Ok(Value::Null)
        }
        (2, "int") => {
            let f: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(raw);
            Ok(Value::Integer(f(call_args[0], call_args[1])))
        }
        (2, "float") => {
            let f: unsafe extern "C" fn(i64, i64) -> f64 = std::mem::transmute(raw);
            Ok(Value::Float(f(call_args[0], call_args[1])))
        }

        // 3 args
        (3, "void") => {
            let f: unsafe extern "C" fn(i64, i64, i64) = std::mem::transmute(raw);
            f(call_args[0], call_args[1], call_args[2]);
            Ok(Value::Null)
        }
        (3, "int") => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> i64 = std::mem::transmute(raw);
            Ok(Value::Integer(f(call_args[0], call_args[1], call_args[2])))
        }
        (3, "float") => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> f64 = std::mem::transmute(raw);
            Ok(Value::Float(f(call_args[0], call_args[1], call_args[2])))
        }

        // 4 args
        (4, "void") => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) = std::mem::transmute(raw);
            f(call_args[0], call_args[1], call_args[2], call_args[3]);
            Ok(Value::Null)
        }
        (4, "int") => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> i64 = std::mem::transmute(raw);
            Ok(Value::Integer(f(
                call_args[0],
                call_args[1],
                call_args[2],
                call_args[3],
            )))
        }
        (4, "float") => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> f64 = std::mem::transmute(raw);
            Ok(Value::Float(f(
                call_args[0],
                call_args[1],
                call_args[2],
                call_args[3],
            )))
        }

        (n, rt) => Err(RuntimeError::new(format!(
            "ffi_call: unsupported combination: {} args with ret_type '{}'. \
             Supported ret_types: \"int\", \"float\", \"void\". Max 4 args.",
            n, rt
        ))),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_load_wrong_arg_type() {
        let result = FfiLib::call_function("ffi_load", &[Value::Integer(42)]);
        assert!(result.is_err());
        // With ffi feature: "must be a string"; without: "requires the 'ffi' feature"
        assert!(result.is_err());
    }

    #[test]
    fn ffi_load_nonexistent_path() {
        let result =
            FfiLib::call_function("ffi_load", &[Value::String("/no/such/lib.so".to_string())]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // With ffi: "ffi_load: could not load..."; without: "requires the 'ffi' feature"
        assert!(!err.message().is_empty());
    }

    #[test]
    fn ffi_close_invalid_handle() {
        // Without ffi feature: error. With ffi feature: silent no-op → Null.
        let result = FfiLib::call_function("ffi_close", &[Value::Integer(9999)]);
        #[cfg(feature = "ffi")]
        assert_eq!(result.unwrap(), Value::Null);
        #[cfg(not(feature = "ffi"))]
        assert!(result.is_err());
    }

    #[test]
    fn ffi_call_invalid_handle() {
        let result = FfiLib::call_function(
            "ffi_call",
            &[
                Value::Integer(8888),
                Value::String("add".to_string()),
                Value::String("int".to_string()),
                Value::Null,
            ],
        );
        assert!(result.is_err());
    }

    #[test]
    fn ffi_call_too_many_args() {
        let big_array = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
            Value::Integer(5),
        ]);
        let result = FfiLib::call_function(
            "ffi_call",
            &[
                Value::Integer(0),
                Value::String("f".to_string()),
                Value::String("void".to_string()),
                big_array,
            ],
        );
        assert!(result.is_err());
    }

    #[test]
    fn ffi_unknown_function() {
        let result = FfiLib::call_function("ffi_unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("Unknown FFI function"));
    }

    #[cfg(all(feature = "ffi", target_os = "linux"))]
    #[test]
    fn ffi_load_and_call_libc() {
        // Load libc.so.6 and call `abs(-7)` which should return 7.
        let load_result =
            FfiLib::call_function("ffi_load", &[Value::String("libc.so.6".to_string())]);
        match load_result {
            Err(_) => return, // libc not directly loadable on this system — skip
            Ok(Value::Integer(handle)) => {
                let call_result = FfiLib::call_function(
                    "ffi_call",
                    &[
                        Value::Integer(handle),
                        Value::String("abs".to_string()),
                        Value::String("int".to_string()),
                        Value::Array(vec![Value::Integer(-7)]),
                    ],
                );
                assert_eq!(call_result.unwrap(), Value::Integer(7));
                let _ = FfiLib::call_function("ffi_close", &[Value::Integer(handle)]);
            }
            Ok(other) => panic!("Expected Integer handle, got {:?}", other),
        }
    }
}
