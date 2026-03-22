use crate::parser::ast::{Parameter, Statement};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

// ---------------------------------------------------------------------------
// FutureHandle — backing type for Value::Future
// ---------------------------------------------------------------------------

/// Shared result cell for an async task.
type FutureInner = Arc<(Mutex<Option<Result<Value, String>>>, Condvar)>;

/// A handle to an asynchronously executing NPL task.
///
/// Created by calling an `async`-defined function without `await`.
/// Resolved by `await`-ing the handle, which blocks the calling thread
/// until the spawned task finishes and stores its result.
pub struct FutureHandle {
    inner: FutureInner,
}

impl FutureHandle {
    /// Create a `(FutureHandle, FutureSender)` pair — the handle is placed in
    /// the parent scope as a `Value::Future`; the sender is moved into the
    /// spawned thread so it can deliver the result.
    pub fn pending() -> (Self, FutureSender) {
        let inner: FutureInner = Arc::new((Mutex::new(None), Condvar::new()));
        (
            FutureHandle { inner: Arc::clone(&inner) },
            FutureSender { inner },
        )
    }

    /// Block until the async task completes, then return its result.
    pub fn resolve(self) -> Result<Value, String> {
        let (lock, cvar) = &*self.inner;
        let guard = lock.lock().unwrap();
        // Wait until the result is Some(…).
        let guard = cvar.wait_while(guard, |r| r.is_none()).unwrap();
        guard.as_ref().unwrap().clone()
    }

    /// Block until the async task completes or the timeout elapses.
    /// Returns `Some(result)` on completion, `None` if the timeout was reached.
    pub fn resolve_with_timeout(
        self,
        timeout: std::time::Duration,
    ) -> Option<Result<Value, String>> {
        let (lock, cvar) = &*self.inner;
        let guard = lock.lock().unwrap();
        let (guard, wait_result) =
            cvar.wait_timeout_while(guard, timeout, |r| r.is_none()).unwrap();
        if wait_result.timed_out() {
            None
        } else {
            guard.clone()
        }
    }
}

impl Clone for FutureHandle {
    fn clone(&self) -> Self {
        FutureHandle { inner: Arc::clone(&self.inner) }
    }
}

impl PartialEq for FutureHandle {
    /// Futures are identity values — two handles are never equal.
    fn eq(&self, _: &Self) -> bool { false }
}

impl std::fmt::Debug for FutureHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<future>")
    }
}

/// Sender side of a future — held exclusively by the spawned thread.
///
/// Dropping `FutureSender` without calling `send` would leave the future
/// pending forever, so `send` should always be called (even on error).
pub struct FutureSender {
    inner: FutureInner,
}

impl FutureSender {
    /// Deliver the task result and wake any thread blocked in `FutureHandle::resolve`.
    pub fn send(self, result: Result<Value, String>) {
        let (lock, cvar) = &*self.inner;
        *lock.lock().unwrap() = Some(result);
        cvar.notify_one();
    }
}

// SAFETY: FutureHandle / FutureSender wrap Arc<(Mutex<…>, Condvar)> — both
// types are Send + Sync through the standard library's guarantees.
unsafe impl Send for FutureHandle {}
unsafe impl Sync for FutureHandle {}
unsafe impl Send for FutureSender {}

// ---------------------------------------------------------------------------
// Value enum
// ---------------------------------------------------------------------------

/// Runtime value representation
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(Arc<str>),
    Char(char),
    Boolean(bool),
    Null,
    Array(Vec<Value>),
    Map(IndexMap<String, Value>),
    Set(Vec<Value>), // Set maintains unique values
    Function(
        String,
        Vec<Parameter>,
        Vec<Statement>,
        HashMap<String, Value>,
    ), // name, params, body, captured_env
    Struct(String, HashMap<String, Value>),
    Enum(String, String, Option<Box<Value>>), // enum_name, variant_name, optional payload
    Result(bool, Box<Value>), // true = Ok(inner), false = Err(inner)
    /// An asynchronously executing task.  Created by calling an `async` function;
    /// resolved by `await`-ing it to block until the task completes.
    Future(FutureHandle),
    /// A reference to a compiled bytecode function by name.
    /// Used exclusively by the bytecode compiler to represent lambda values
    /// on the stack, so HOF dispatch can distinguish function references from
    /// plain strings without risking accidental mis-dispatch.
    FunctionRef(String),
    /// Raw byte buffer, e.g. from reading binary files or `bytes_from_hex`.
    Bytes(Vec<u8>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Value::Map(map) => {
                // IndexMap preserves insertion order — no sort needed.
                let items: Vec<String> =
                    map.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Set(set) => {
                let items: Vec<String> = set.iter().map(|v| v.to_string()).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Function(name, _, _, _) => write!(f, "<function {}>", name),
            Value::Struct(name, fields) => {
                let mut pairs: Vec<(&String, &Value)> = fields.iter().collect();
                pairs.sort_by_key(|(k, _)| k.as_str());
                let field_strs: Vec<String> = pairs.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{}({})", name, field_strs.join(", "))
            }
            Value::Enum(name, variant, payload) => {
                if let Some(inner) = payload {
                    write!(f, "{}.{}({})", name, variant, inner)
                } else {
                    write!(f, "{}.{}", name, variant)
                }
            }
            Value::Future(_) => write!(f, "<future>"),
            Value::FunctionRef(name) => write!(f, "<fn:{}>", name),
            Value::Bytes(b) => write!(f, "<bytes:{}>", b.len()),
            Value::Result(ok, inner) => {
                if *ok {
                    write!(f, "Ok({})", inner)
                } else {
                    write!(f, "Err({})", inner)
                }
            }
        }
    }
}

impl Value {
    /// Check if a value is in a set (for uniqueness checking)
    #[allow(dead_code)]
    pub fn set_contains(set: &[Value], value: &Value) -> bool {
        set.iter().any(|v| v == value)
    }

    /// Return a human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Char(_) => "char",
            Value::Boolean(_) => "bool",
            Value::Null => "null",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
            Value::Set(_) => "set",
            Value::Function(_, _, _, _) => "function",
            Value::Struct(_, _) => "struct",
            Value::Enum(_, _, _) => "enum",
            Value::Result(_, _) => "result",
            Value::Future(_) => "future",
            Value::FunctionRef(_) => "function_ref",
            Value::Bytes(_) => "bytes",
        }
    }
}
