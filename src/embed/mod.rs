/// Task 28.2 — Embedding API (Lua-Style)
///
/// Provides a simple high-level Rust API for embedding the Txt-code interpreter
/// in other Rust programs, and a C-compatible ABI for FFI embedding from C/C++/Python.
///
/// # Rust API
///
/// ```text
/// use txtcode::embed::TxtcodeEngine;
///
/// let mut engine = TxtcodeEngine::new();
/// let result = engine.eval("1 + 1").unwrap();
/// assert_eq!(result.as_integer(), Some(2));
/// ```
///
/// # C API (via `cdylib`)
///
/// ```text
/// TxtcodeEngine *e = txtcode_new();
/// txtcode_set_int(e, "x", 42);
/// txtcode_eval(e, "store → y → x * 2");
/// long long y = txtcode_get_int(e, "y");  // 84
/// txtcode_free(e);
/// ```

use crate::lexer::lexer::Lexer;
use std::sync::Arc;
use crate::parser::parser::Parser;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::vm::VirtualMachine;

/// High-level embedding API for the Txt-code interpreter.
pub struct TxtcodeEngine {
    vm: VirtualMachine,
    /// Error code from the most recent `eval` call (None if last call succeeded).
    last_error_code: Option<u32>,
}

impl TxtcodeEngine {
    /// Create a new engine with default (unprivileged) settings.
    pub fn new() -> Self {
        Self {
            vm: VirtualMachine::new(),
            last_error_code: None,
        }
    }

    /// Parse and run `source`, returning the last evaluated value.
    pub fn eval(&mut self, source: &str) -> Result<Value, RuntimeError> {
        let result = self.eval_inner(source);
        match &result {
            Ok(_) => self.last_error_code = None,
            Err(e) => {
                self.last_error_code = e.code.as_ref().map(|c| {
                    c.as_str().trim_start_matches('E').parse::<u32>().unwrap_or(0)
                });
            }
        }
        result
    }

    fn eval_inner(&mut self, source: &str) -> Result<Value, RuntimeError> {
        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer
            .tokenize()
            .map_err(|e| RuntimeError::new(e))?;
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| RuntimeError::new(e))?;
        self.vm.interpret_repl(&program)
    }

    /// Evaluate source and return the result as a string.
    ///
    /// Convenience wrapper for host programs that only need string output.
    /// Returns `Err(String)` on any error (parse, runtime, permission).
    pub fn eval_string(&mut self, source: &str) -> Result<String, String> {
        self.eval(source)
            .map(|v| format!("{}", v))
            .map_err(|e| e.message().to_string())
    }

    /// Return the numeric error code from the most recent `eval` call.
    ///
    /// Returns `None` if the last call succeeded or no call has been made.
    /// The code corresponds to the `ErrorCode` enum (e.g. 1 = permission denied,
    /// 10 = undefined variable, 11 = type mismatch, etc.)
    pub fn last_error_code(&self) -> Option<u32> {
        self.last_error_code
    }

    /// Set a global variable accessible from scripts.
    pub fn set(&mut self, name: &str, value: Value) {
        self.vm.define_global(name.to_string(), value);
    }

    /// Read a global variable that was set by a script or via [`set`].
    pub fn get(&self, name: &str) -> Option<Value> {
        self.vm.globals_snapshot().remove(name)
    }

    /// Register a host function callable from scripts.
    ///
    /// The closure receives evaluated arguments and must return a `Value`.
    /// Errors should be returned as `Value::String(Arc::from(message))` or similar.
    pub fn register_fn<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&[Value]) -> Value + Send + Sync + 'static,
    {
        // Wrap in a Value::NativeFunction (stored as a closure in globals)
        // We represent host functions as Value::Function stubs; at call time
        // the VM hits the native registry.  A simpler approach: store the fn
        // in globals under a special key and expose it as a lambda wrapper.
        //
        // Implementation: store as a callable in the globals map using a
        // dedicated NativeFunction value variant path.  Because Value does not
        // currently have a NativeFunction variant, we use a thread-local
        // registry keyed by name, and inject a zero-arg Function declaration
        // that the eval loop will pick up through the stdlib path.
        //
        // For now we register into a global table that stdlib's call path
        // checks before reaching the built-in dispatch.
        NATIVE_REGISTRY.lock().unwrap().insert(name.to_string(), Box::new(f));
        // Define a placeholder in the VM scope so the parser/resolver sees
        // the name as defined.
        self.vm.define_global(
            name.to_string(),
            Value::String(Arc::from(format!("__native_fn::{}", name))),
        );
    }
}

impl Default for TxtcodeEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Native function registry ──────────────────────────────────────────────────

type NativeFn = Box<dyn Fn(&[Value]) -> Value + Send + Sync + 'static>;

lazy_static::lazy_static! {
    pub(crate) static ref NATIVE_REGISTRY: std::sync::Mutex<std::collections::HashMap<String, NativeFn>> =
        std::sync::Mutex::new(std::collections::HashMap::new());
}

/// Look up and call a registered native function.  Returns `None` if not found.
pub fn call_native(name: &str, args: &[Value]) -> Option<Value> {
    let guard = NATIVE_REGISTRY.lock().unwrap();
    guard.get(name).map(|f| f(args))
}

// ── C-compatible API ──────────────────────────────────────────────────────────

/// Allocate a new `TxtcodeEngine` and return an opaque pointer.
///
/// The caller is responsible for calling [`txtcode_free`] when done.
///
/// # Safety
/// The returned pointer is valid until passed to `txtcode_free`.
#[no_mangle]
pub unsafe extern "C" fn txtcode_new() -> *mut TxtcodeEngine {
    Box::into_raw(Box::new(TxtcodeEngine::new()))
}

/// Evaluate a null-terminated source string.
///
/// Return codes:
///   0  = success
///  -1  = parse error
///  -2  = runtime error
///  -3  = permission denied
///
/// # Safety
/// `engine` must be a valid pointer from `txtcode_new`, and `source` must be
/// a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn txtcode_eval(engine: *mut TxtcodeEngine, source: *const i8) -> i32 {
    let engine = &mut *engine;
    let source = unsafe { std::ffi::CStr::from_ptr(source) }
        .to_str()
        .unwrap_or("");
    match engine.eval(source) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("[txtcode] eval error: {}", e.message());
            // Classify the error for C callers.
            match &e.code {
                Some(code) => {
                    let s = code.as_str();
                    // E0001–E0004 = permission/capability/rate/intent
                    if matches!(s, "E0001" | "E0002" | "E0003" | "E0004") {
                        -3
                    } else {
                        -2
                    }
                }
                None => {
                    let msg = e.message();
                    // Parse errors come from the lexer/parser before reaching VM
                    if msg.contains("parse") || msg.contains("unexpected token")
                        || msg.contains("expected") || msg.contains("syntax") {
                        -1
                    } else {
                        -2
                    }
                }
            }
        }
    }
}

/// Set an integer global variable.
///
/// # Safety
/// `engine` must be valid, `name` must be a null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn txtcode_set_int(engine: *mut TxtcodeEngine, name: *const i8, value: i64) {
    let engine = &mut *engine;
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    engine.set(name, Value::Integer(value));
}

/// Get an integer global variable.  Returns 0 if not set or not an integer.
///
/// # Safety
/// `engine` must be valid, `name` must be a null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn txtcode_get_int(engine: *mut TxtcodeEngine, name: *const i8) -> i64 {
    let engine = &mut *engine;
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    match engine.get(name) {
        Some(Value::Integer(i)) => i,
        _ => 0,
    }
}

/// Set a string global variable.
///
/// # Safety
/// `engine` must be valid; `name` and `value` must be null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn txtcode_set_string(
    engine: *mut TxtcodeEngine,
    name: *const i8,
    value: *const i8,
) {
    let engine = &mut *engine;
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    let value = unsafe { std::ffi::CStr::from_ptr(value) }
        .to_str()
        .unwrap_or("");
    engine.set(name, Value::String(Arc::from(value.to_string())));
}

/// Set a string global variable with an explicit byte length.
///
/// Unlike [`txtcode_set_string`], this variant handles strings that contain
/// embedded null bytes.  `len` is the number of bytes in `value` (not including
/// any trailing null).  Invalid UTF-8 sequences are replaced with U+FFFD.
///
/// # Safety
/// `engine` must be valid; `name` must be a null-terminated string; `value`
/// must point to at least `len` valid bytes.
#[no_mangle]
pub unsafe extern "C" fn txtcode_set_string_n(
    engine: *mut TxtcodeEngine,
    name: *const i8,
    value: *const i8,
    len: usize,
) {
    let engine = &mut *engine;
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    let bytes = unsafe { std::slice::from_raw_parts(value as *const u8, len) };
    let s = String::from_utf8_lossy(bytes).into_owned();
    engine.set(name, Value::String(Arc::from(s)));
}

/// Get a string global variable.  Returns a null pointer if not set.
///
/// The returned string is heap-allocated; free it with [`txtcode_free_string`].
///
/// # Safety
/// `engine` must be valid, `name` must be a null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn txtcode_get_string(
    engine: *mut TxtcodeEngine,
    name: *const i8,
) -> *mut i8 {
    let engine = &mut *engine;
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    match engine.get(name) {
        Some(Value::String(s)) => {
            match std::ffi::CString::new(s.as_ref()) {
                Ok(cs) => cs.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        _ => std::ptr::null_mut(),
    }
}

/// Free a string returned by [`txtcode_get_string`].
///
/// # Safety
/// `ptr` must be a pointer previously returned by `txtcode_get_string`.
#[no_mangle]
pub unsafe extern "C" fn txtcode_free_string(ptr: *mut i8) {
    if !ptr.is_null() {
        drop(unsafe { std::ffi::CString::from_raw(ptr) });
    }
}

/// Free a `TxtcodeEngine` allocated by `txtcode_new`.
///
/// # Safety
/// `engine` must be a valid pointer from `txtcode_new` and must not be used
/// after this call.
#[no_mangle]
pub unsafe extern "C" fn txtcode_free(engine: *mut TxtcodeEngine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}
