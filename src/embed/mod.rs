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
use crate::builder::{BuildConfig, Builder};
use crate::lexer::lexer::Lexer;
use crate::parser::parser::Parser;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::validator::Validator;
use std::sync::Arc;

/// High-level embedding API for the Txt-code interpreter.
pub struct TxtcodeEngine {
    vm: crate::runtime::vm::VirtualMachine,
    /// Error code from the most recent `eval` call (None if last call succeeded).
    last_error_code: Option<u32>,
}

impl TxtcodeEngine {
    /// Create a new engine with default (unprivileged) settings.
    pub fn new() -> Self {
        Self {
            vm: Builder::create_vm(&BuildConfig::default()),
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
            .map_err(RuntimeError::new)?;
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(RuntimeError::new)?;
        // P.1: run the full validator pipeline before execution.
        // Without this, host applications embedding via C ABI received zero
        // semantic validation and no restriction checking.
        Validator::validate_program(&program)
            .map_err(|e| RuntimeError::new(e.to_string()))?;
        self.vm.interpret_repl(&program)
    }

    /// Create a sandboxed engine (applies OS-level seccomp/prctl hardening).
    ///
    /// Use when running untrusted scripts from a host application.
    /// The sandbox is applied once at construction time. Default [`new()`]
    /// remains unprivileged but now includes full semantic validation (P.1).
    pub fn with_sandbox() -> Self {
        // Non-fatal: language-level permissions still apply if OS sandbox fails.
        if let Err(e) = crate::runtime::sandbox::apply_sandbox(true) {
            eprintln!("[txtcode] warning: OS sandbox could not be applied: {}", e);
        }
        Self {
            vm: Builder::create_vm(&BuildConfig::default()),
            last_error_code: None,
        }
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
    ///
    /// Functions are stored per-engine, so multiple engines can register
    /// different implementations under the same name without collision.
    pub fn register_fn<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&[Value]) -> Value + Send + Sync + 'static,
    {
        // Store in the per-VM registry so different engine instances don't share
        // a global table and cannot overwrite each other's functions.
        self.vm.register_native_fn(name, f);
        // Define a placeholder in the VM scope so the resolver sees the name as
        // defined and routes calls through the "__native_fn::" fast path.
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
