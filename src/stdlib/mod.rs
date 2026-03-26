use std::sync::Arc;
pub mod bytes;
pub mod capabilities;
pub mod db;
pub mod core;
pub mod crypto;
pub mod errors;
pub mod ffi;
pub mod function_executor;
pub mod plugin;
pub mod io;
pub mod json;
pub mod log;
#[cfg(feature = "net")]
pub mod net;
pub mod path;
pub mod permission_checker;
pub mod regex;
pub mod sys;
pub mod template;
pub mod test;
pub mod time;
pub mod tools;
pub mod url;
pub mod wasm_exec;

// ── Group 26.3: Async cancellation token registry ─────────────────────────
// Maps integer token IDs → Arc<AtomicBool> (true = cancelled).
// Tokens are created by async_cancel_token() and checked by is_cancelled().
lazy_static::lazy_static! {
    static ref CANCEL_TOKENS: std::sync::Mutex<
        std::collections::HashMap<i64, std::sync::Arc<std::sync::atomic::AtomicBool>>
    > = std::sync::Mutex::new(std::collections::HashMap::new());
    static ref CANCEL_TOKEN_NEXT_ID: std::sync::Mutex<i64> = std::sync::Mutex::new(1);
}

// ── P.1: O(1) stdlib dispatch table ───────────────────────────────────────
// Maps exact function names to a module code for O(1) routing.
// Functions that need special executor/event-loop handling (db_transaction,
// http_serve, ws_serve, async_http_*, async_cancel_*, await_all/any,
// grant_capability) are NOT in this table — they stay in the explicit branches.
const M_CORE:     u8 =  0;
const M_CRYPTO:   u8 =  1;
const M_IO:       u8 =  2;
const M_SYS:      u8 =  3;
const M_TIME:     u8 =  4;
const M_LOG:      u8 =  5;
const M_TEST:     u8 =  6;
const M_DB:       u8 =  7;
const M_FFI:      u8 =  8;
const M_PLUGIN:   u8 =  9;
const M_WASM:     u8 = 10;
const M_TEMPLATE: u8 = 11;
const M_ERROR:    u8 = 12;
const M_NET:      u8 = 13;
const M_BYTES:    u8 = 14;

lazy_static::lazy_static! {
    static ref STDLIB_DISPATCH: std::collections::HashMap<&'static str, u8> = {
        let mut m = std::collections::HashMap::with_capacity(256);
        // Core — exact names (prefix variants str_*, math_*, array_*, set_*, to_*
        // are still handled by starts_with checks in the fallback path)
        for n in &["len", "length", "type", "input", "print", "max", "min", "set",
                   "map", "filter", "reduce", "find", "range", "enumerate",
                   "zip", "chain", "sin", "cos", "tan", "sqrt", "log", "pow",
                   "abs", "floor", "ceil", "round", "split", "join", "replace",
                   "trim", "substring", "indexOf", "startsWith", "endsWith",
                   "toUpper", "toLower", "sort", "reverse", "concat",
                   "ok", "err", "is_ok", "is_err", "unwrap", "unwrap_or",
                   "base32_encode", "base32_decode", "html_escape", "csv_to_string",
                   "xml_decode", "xml_parse", "xml_stringify", "xml_encode",
                   "math_random_float", "format", "string", "int", "float", "bool",
                   "str_build", "array_slice"] {
            m.insert(*n, M_CORE);
        }
        // Crypto
        for n in &["md5", "encrypt", "decrypt", "base64_encode", "base64_decode",
                   "hmac_sha256", "uuid_v4", "secure_compare", "pbkdf2",
                   "bcrypt_hash", "bcrypt_verify", "ed25519_sign", "ed25519_verify",
                   "rsa_generate", "rsa_sign", "rsa_verify",
                   "jwt_sign", "jwt_verify", "jwt_decode"] {
            m.insert(*n, M_CRYPTO);
        }
        // IO
        for n in &["is_file", "is_dir", "mkdir", "rmdir", "delete",
                   "append_file", "copy_file", "move_file", "rename_file",
                   "temp_file", "watch_file", "symlink_create",
                   "zip_create", "zip_extract", "csv_write",
                   "async_read_file", "async_write_file",
                   "csv_stream_reader", "csv_stream_writer",
                   "csv_read_row", "csv_write_row", "csv_stream_close"] {
            m.insert(*n, M_IO);
        }
        // System
        for n in &["getenv", "setenv", "platform", "arch", "exec",
                   "exec_status", "exec_lines", "exec_json", "exit", "args",
                   "cwd", "env_list", "signal_send", "pipe_exec", "exec_pipe",
                   "which", "is_root", "cpu_count", "memory_available",
                   "disk_space", "os_name", "os_version", "cli_parse",
                   "proc_run", "proc_pipe", "chdir", "pid", "user", "home",
                   "uid", "gid", "spawn", "kill", "wait"] {
            m.insert(*n, M_SYS);
        }
        // Time
        for n in &["now", "sleep", "async_sleep", "format_time", "time_format",
                   "parse_time", "time_parse", "now_utc", "now_local",
                   "parse_datetime", "format_datetime", "datetime_add", "datetime_diff"] {
            m.insert(*n, M_TIME);
        }
        // Log
        m.insert("log", M_LOG);
        // Test
        m.insert("expect_error", M_TEST);
        // DB (NOT db_transaction — it needs executor for the closure call)
        for n in &["db_open", "db_exec", "db_close", "db_connect", "db_query",
                   "db_execute", "db_commit", "db_rollback", "db_schema", "db_tables"] {
            m.insert(*n, M_DB);
        }
        // FFI
        for n in &["ffi_load", "ffi_call", "ffi_close"] { m.insert(*n, M_FFI); }
        // Plugin
        for n in &["plugin_load", "plugin_functions", "plugin_call"] { m.insert(*n, M_PLUGIN); }
        // WASM
        for n in &["wasm_load", "wasm_call", "wasm_close"] { m.insert(*n, M_WASM); }
        // Template
        m.insert("template_render", M_TEMPLATE);
        // Error constructors
        for n in &["FileNotFoundError", "PermissionError", "NetworkError",
                   "ParseError", "TypeError", "ValueError", "IndexError", "TimeoutError"] {
            m.insert(*n, M_ERROR);
        }
        // Net exact names (most go through http/ws/tcp prefix check)
        for n in &["udp_send", "resolve", "tls_connect", "websocket_connect",
                   "dns_resolve", "net_ping", "net_port_open"] {
            m.insert(*n, M_NET);
        }
        // Bytes / gzip
        for n in &["gzip_compress", "gzip_decompress",
                   "gzip_compress_string", "gzip_decompress_string"] {
            m.insert(*n, M_BYTES);
        }
        m
    };

    /// Functions NOT in STDLIB_DISPATCH because they need executor/closure context
    /// or are handled by the fallback prefix-routing path.
    static ref STDLIB_EXTRA_NAMES: std::collections::HashSet<&'static str> = [
        // Executor-dependent
        "db_transaction", "http_serve", "ws_serve",
        "async_http_get", "async_http_post",
        "await_all", "await_any",
        "async_run", "async_run_scoped", "async_run_timeout",
        "async_cancel_token", "async_cancel", "is_cancelled", "async_cancel_token_drop",
        "grant_capability", "revoke_capability",
        // Aliases / common builtins not in dispatch
        "str", "println", "print", "hash_sha256",
        // Commonly used in tests / imported from core
        "double", "triple",
        // Prefix-routed — representative well-known names
        "http_request", "http_patch", "http_put", "http_delete",
        "ws_connect", "ws_send", "ws_recv", "ws_close",
        "http_get", "http_post",
        "tcp_connect", "tcp_send", "tcp_recv", "tcp_close",
        "async_http_get", "async_http_post",
        "read_file", "write_file", "delete_file", "file_exists",
        "list_dir", "make_dir", "path_join", "path_basename", "path_dirname",
        "json_parse", "json_stringify",
        // Test assertions
        "assert", "assert_eq", "assert_ne", "assert_error", "assert_type",
        "assert_contains", "assert_approx",
        // Misc builtins
        "self", "null",
    ].iter().copied().collect();

    /// All known stdlib function names (STDLIB_DISPATCH + extras).
    ///
    /// Used by the validator's undefined-variable check to suppress false positives.
    static ref STDLIB_ALL_NAMES: std::collections::HashSet<&'static str> = {
        let mut names: std::collections::HashSet<&'static str> =
            STDLIB_DISPATCH.keys().copied().collect();
        for name in STDLIB_EXTRA_NAMES.iter() {
            names.insert(name);
        }
        names
    };
}

/// Return the set of all known stdlib function/built-in names.
///
/// This is the authoritative source for the validator's undefined-variable pass.
/// It is built from the runtime dispatch table (`STDLIB_DISPATCH`) plus a supplemental
/// list of executor-dependent and prefix-routed functions.
pub fn stdlib_function_names() -> &'static std::collections::HashSet<&'static str> {
    &STDLIB_ALL_NAMES
}

/// Return true if `name` matches a well-known stdlib name prefix.
///
/// Prefix-routed functions (str_*, math_*, http_*, etc.) cannot be enumerated
/// statically. This predicate is used by the validator as a fallback after the
/// exact-name check fails, preventing false "undefined variable" warnings.
pub fn is_stdlib_prefix(name: &str) -> bool {
    name.starts_with("str_")
        || name.starts_with("math_")
        || name.starts_with("array_")
        || name.starts_with("set_")
        || name.starts_with("to_")
        || name.starts_with("from_")
        || name.starts_with("http_")
        || name.starts_with("ws_")
        || name.starts_with("tcp_")
        || name.starts_with("udp_")
        || name.starts_with("csv_")
        || name.starts_with("toml_")
        || name.starts_with("yaml_")
        || name.starts_with("sha")
        || name.starts_with("crypto_")
        || name.starts_with("async_")
        || name.starts_with("await_")
        || name.starts_with("db_")
        || name.starts_with("net_")
        || name.starts_with("ffi_")
        || name.starts_with("plugin_")
        || name.starts_with("wasm_")
        || name.starts_with("log_")
        || name.starts_with("time_")
        || name.starts_with("path_")
        || name.starts_with("json_")
        || name.starts_with("xml_")
        || name.starts_with("gzip_")
        || name.starts_with("jwt_")
        || name.starts_with("rsa_")
        || name.starts_with("ed25519_")
        || name.starts_with("test_")
        || name.starts_with("assert_")
}

pub use bytes::BytesLib;
pub use capabilities::{CapabilityExecutor, CapabilityLib};
pub use db::DbLib;
pub use core::CoreLib;
pub use crypto::CryptoLib;
pub use ffi::FfiLib;
pub use plugin::PluginLib;
pub use function_executor::FunctionExecutor;
pub use io::IOLib;
pub use json::JsonLib;
pub use log::LogLib;
#[cfg(feature = "net")]
pub use net::NetLib;
pub use path::PathLib;
pub use permission_checker::PermissionChecker;
pub use regex::RegexLib;
pub use sys::SysLib;
pub use template::TemplateLib;
pub use test::TestLib;
pub use time::TimeLib;
pub use tools::ToolLib;
pub use url::UrlLib;

/// Standard library function dispatcher
pub struct StdLib;

impl StdLib {
    /// Call any standard library function
    /// exec_allowed: whether exec() function is allowed (for safe mode)
    /// executor: optional function executor for higher-order functions
    /// If executor implements PermissionChecker, it will be used for permission checking
    pub fn call_function<E>(
        name: &str,
        args: &[crate::runtime::Value],
        exec_allowed: bool,
        executor: Option<&mut E>,
    ) -> Result<crate::runtime::Value, crate::runtime::RuntimeError>
    where
        E: FunctionExecutor,
    {
        // Try to extract permission checker from executor if it implements PermissionChecker
        // Since we can't use &mut and & at the same time, we'll pass None here
        // and use call_function_with_permission_checker when available
        Self::call_function_with_permission_checker_internal(
            name,
            args,
            exec_allowed,
            executor,
            None,
        )
    }

    /// Call function with explicit permission checker (avoids borrow checker issues)
    pub fn call_function_with_permission_checker<E>(
        name: &str,
        args: &[crate::runtime::Value],
        exec_allowed: bool,
        executor: Option<&mut E>,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> Result<crate::runtime::Value, crate::runtime::RuntimeError>
    where
        E: FunctionExecutor,
    {
        Self::call_function_with_permission_checker_internal(
            name,
            args,
            exec_allowed,
            executor,
            permission_checker,
        )
    }

    /// Call function with executor that implements both FunctionExecutor and PermissionChecker.
    ///
    /// Permission checking is done inline here (immutable borrow) before the mutable call,
    /// which avoids the Rust borrow-checker conflict between `&dyn PermissionChecker` and
    /// `&mut E` pointing at the same value.
    ///
    /// The function → resource mapping is delegated to [`crate::runtime::permission_map`],
    /// the single source of truth shared with the AST VM's expression evaluator.
    pub fn call_function_with_combined_traits<E>(
        name: &str,
        args: &[crate::runtime::Value],
        exec_allowed: bool,
        executor: Option<&mut E>,
    ) -> Result<crate::runtime::Value, crate::runtime::RuntimeError>
    where
        E: FunctionExecutor + PermissionChecker,
    {
        // Upfront permission check using an immutable reborrow of the executor.
        // Extract the real scope from args (same logic as AST VM's function_calls.rs)
        // so the check is scope-aware rather than passing None for all calls.
        if let Some(ref exec) = executor {
            if let Some(resource) =
                crate::runtime::permission_map::map_function_to_permission(name)
            {
                let scope = crate::runtime::permission_map::extract_permission_scope(&resource, args);
                exec.check_permission(&resource, scope.as_deref())?;
            }
        }

        // Now call the internal routing without a separate permission_checker reference
        // (resources gated above; stdlib modules receive None and skip their own check).
        Self::call_function_with_permission_checker_internal(
            name,
            args,
            exec_allowed,
            executor,
            None,
        )
    }

    /// Internal function with explicit permission checker
    fn call_function_with_permission_checker_internal<E: FunctionExecutor>(
        name: &str,
        args: &[crate::runtime::Value],
        exec_allowed: bool,
        executor: Option<&mut E>,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> Result<crate::runtime::Value, crate::runtime::RuntimeError> {
        let effective_permission_checker = permission_checker;

        // Extract deterministic overrides from the executor (if present)
        let time_override = executor.as_ref().and_then(|e| e.deterministic_time());
        let seed_override = executor.as_ref().and_then(|e| e.deterministic_random_seed());

        // ── P.1: O(1) fast-path dispatch for exact-name functions ─────────────
        // Functions that need special executor/event-loop handling are excluded
        // from this table and fall through to the explicit branches below.
        if let Some(&module) = STDLIB_DISPATCH.get(name) {
            return match module {
                M_CORE     => CoreLib::call_function(name, args, executor),
                M_CRYPTO   => CryptoLib::call_function(name, args, seed_override),
                M_IO       => IOLib::call_function(name, args, effective_permission_checker),
                M_SYS      => SysLib::call_function(name, args, exec_allowed, effective_permission_checker),
                M_TIME     => TimeLib::call_function(name, args, time_override),
                M_LOG      => LogLib::call_function(name, args),
                M_TEST     => TestLib::call_function(name, args),
                M_DB       => DbLib::call_function(name, args, effective_permission_checker),
                M_FFI      => {
                    #[cfg(feature = "ffi")]
                    { FfiLib::call_function(name, args, effective_permission_checker) }
                    #[cfg(not(feature = "ffi"))]
                    Err(crate::runtime::RuntimeError::new(format!(
                        "FFI function '{}' requires the 'ffi' feature. \
                         Rebuild with: cargo build --features ffi", name)))
                }
                M_PLUGIN   => PluginLib::call_function(name, args, effective_permission_checker),
                M_WASM     => crate::stdlib::wasm_exec::call_function(name, args),
                M_TEMPLATE => TemplateLib::call_function(name, args),
                M_ERROR    => errors::ErrorLib::call_function(name, args),
                M_NET      => {
                    #[cfg(feature = "net")]
                    { NetLib::call_function(name, args, effective_permission_checker) }
                    #[cfg(not(feature = "net"))]
                    Err(crate::runtime::RuntimeError::new(format!(
                        "Network function '{}' requires the 'net' feature. \
                         Rebuild with: cargo build --features net", name)))
                }
                M_BYTES    => BytesLib::call_function(name, args),
                _          => unreachable!("Unknown module code for '{}'", name),
            };
        }
        // ─────────────────────────────────────────────────────────────────────

        // Route to appropriate library
        if name.starts_with("str_")
            || name.starts_with("math_")
            || name.starts_with("array_")
            || name.starts_with("set_")
            || name == "set"
            || name == "len"
            || name == "type"
            || name.starts_with("to_")
            || name == "string"
            || name == "int"
            || name == "float"
            || name == "bool"
            || name == "input"
            || name == "print"
            || name == "max"
            || name == "min"
            || name == "map"
            || name == "filter"
            || name == "reduce"
            || name == "find"
            || name == "range"
            || name == "enumerate"
            || name == "zip"
            || name == "chain"
            || name == "sin"
            || name == "cos"
            || name == "tan"
            || name == "sqrt"
            || name == "log"
            || name == "pow"
            || name == "abs"
            || name == "floor"
            || name == "ceil"
            || name == "round"
            || name == "split"
            || name == "join"
            || name == "replace"
            || name == "trim"
            || name == "substring"
            || name == "indexOf"
            || name == "startsWith"
            || name == "endsWith"
            || name == "toUpper"
            || name == "toLower"
            || name == "sort"
            || name == "reverse"
            || name == "concat"
            || name == "ok"
            || name == "err"
            || name == "is_ok"
            || name == "is_err"
            || name == "unwrap"
            || name == "unwrap_or"
            || name == "base32_encode"
            || name == "base32_decode"
            || name == "html_escape"
            || name.starts_with("toml_")
            || (name.starts_with("csv_") && name != "csv_write")
            || name == "csv_to_string"
            || name == "xml_decode"
            || name == "xml_parse"
            || name == "xml_stringify"
            || name == "xml_encode"
            || name.starts_with("yaml_")
            || name == "math_random_float"
            || name == "format"
        {
            CoreLib::call_function(name, args, executor)
        } else if name.starts_with("sha")
            || name.starts_with("crypto_")
            || name == "encrypt"
            || name == "decrypt"
            || name == "md5"
            || name == "base64_encode"
            || name == "base64_decode"
            || name == "hmac_sha256"
            || name == "uuid_v4"
            || name == "secure_compare"
            || name == "pbkdf2"
            || name == "bcrypt_hash"
            || name == "bcrypt_verify"
            || name == "ed25519_sign"
            || name == "ed25519_verify"
            || name == "rsa_generate"
            || name == "rsa_sign"
            || name == "rsa_verify"
            || name == "jwt_sign"
            || name == "jwt_verify"
            || name == "jwt_decode"
        {
            CryptoLib::call_function(name, args, seed_override)
        } else if name == "db_transaction" {
            // R.1: db_transaction(conn, handler) — auto-rollback closure API.
            // Requires an executor context (to call the handler closure).
            if let Some(exec) = executor {
                return crate::stdlib::db::DbLib::transaction_with_executor(
                    args, exec, effective_permission_checker,
                );
            }
            // Fallback: no executor — legacy BEGIN-only mode.
            return crate::stdlib::db::DbLib::call_function(name, args, effective_permission_checker);
        } else if name == "http_serve" {
            #[cfg(feature = "net")]
            {
                if let Some(exec) = executor {
                    return crate::stdlib::net::NetLib::serve_with_executor(
                        args, exec, effective_permission_checker,
                    );
                }
                return Err(crate::runtime::RuntimeError::new(
                    "http_serve: requires an executor context (call from within VM)".to_string(),
                ));
            }
            #[cfg(not(feature = "net"))]
            return Err(crate::runtime::RuntimeError::new(
                "http_serve requires the 'net' feature. Rebuild with: cargo build --features net".to_string(),
            ))
        } else if name == "ws_serve" {
            #[cfg(feature = "net")]
            {
                if let Some(exec) = executor {
                    return crate::stdlib::net::NetLib::serve_ws_with_executor(
                        args, exec, effective_permission_checker,
                    );
                }
                return Err(crate::runtime::RuntimeError::new(
                    "ws_serve: requires an executor context (call from within VM)".to_string(),
                ));
            }
            #[cfg(not(feature = "net"))]
            return Err(crate::runtime::RuntimeError::new(
                "ws_serve requires the 'net' feature. Rebuild with: cargo build --features net".to_string(),
            ))
        } else if name == "async_http_get" || name == "async_http_post" {
            // Task 26.2: Non-blocking HTTP wrappers.
            // Submits the blocking http_get/http_post call to the event loop (or a thread)
            // and returns a Future handle immediately.
            if let Some(checker) = effective_permission_checker {
                use crate::runtime::permissions::PermissionResource;
                checker.check_permission(&PermissionResource::Network("connect".to_string()), None)?;
            }
            let url = match args.first() {
                Some(crate::runtime::Value::String(s)) => s.clone(),
                _ => return Err(crate::runtime::RuntimeError::new(format!("{}: first argument must be a URL string", name))),
            };
            let body_arg = if name == "async_http_post" {
                args.get(1).cloned()
            } else {
                None
            };
            let fn_name = name.to_string();
            let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
            let task = move || {
                let result: Result<crate::runtime::Value, String> = (|| -> Result<crate::runtime::Value, crate::runtime::RuntimeError> {
                    #[cfg(feature = "net")]
                    {
                        if fn_name == "async_http_get" {
                            crate::stdlib::net::NetLib::call_function("http_get", &[crate::runtime::Value::String(Arc::from(url))], None)
                        } else {
                            let body = body_arg.unwrap_or(crate::runtime::Value::String(Arc::from("{}".to_string())));
                            crate::stdlib::net::NetLib::call_function("http_post", &[crate::runtime::Value::String(Arc::from(url)), body], None)
                        }
                    }
                    #[cfg(not(feature = "net"))]
                    Err(crate::runtime::RuntimeError::new(format!("{}: requires the 'net' feature", fn_name)))
                })()
                .map_err(|e| e.to_string());
                sender.send(result);
            };
            if crate::runtime::event_loop::is_enabled() {
                crate::runtime::event_loop::submit(Box::new(task));
            } else {
                std::thread::spawn(task);
            }
            return Ok(crate::runtime::Value::Future(handle));
        } else if name.starts_with("http")
            || name.starts_with("tcp")
            || name == "udp_send"
            || name == "resolve"
            || name == "tls_connect"
            || name.starts_with("ws_")
            || name == "websocket_connect"
            || name == "dns_resolve"
            || name == "net_ping"
            || name == "net_port_open"
        {
            #[cfg(feature = "net")]
            return NetLib::call_function(name, args, effective_permission_checker);
            #[cfg(not(feature = "net"))]
            return Err(crate::runtime::RuntimeError::new(format!(
                "Network function '{}' requires the 'net' feature. \
                 Rebuild with: cargo build --features net",
                name
            )))
        } else if name.starts_with("read")
            || name.starts_with("write")
            || name.starts_with("file")
            || name.starts_with("list")
            || name == "is_file"
            || name == "is_dir"
            || name == "mkdir"
            || name == "rmdir"
            || name == "delete"
            || name == "append_file"
            || name == "copy_file"
            || name == "move_file"
            || name == "rename_file"
            || name == "temp_file"
            || name == "watch_file"
            || name == "symlink_create"
            || name == "zip_create"
            || name == "zip_extract"
            || name == "csv_write"
            || name == "async_read_file"
            || name == "async_write_file"
            || name == "csv_stream_reader"
            || name == "csv_stream_writer"
            || name == "csv_read_row"
            || name == "csv_write_row"
            || name == "csv_stream_close"
        {
            IOLib::call_function(name, args, effective_permission_checker)
        } else if name == "getenv"
            || name == "setenv"
            || name == "platform"
            || name == "arch"
            || name == "exec"
            || name == "exec_status"
            || name == "exec_lines"
            || name == "exec_json"
            || name == "exit"
            || name == "args"
            || name == "cwd"
            || name == "env_list"
            || name == "signal_send"
            || name == "pipe_exec"
            || name == "exec_pipe"
            || name == "which"
            || name == "is_root"
            || name == "cpu_count"
            || name == "memory_available"
            || name == "disk_space"
            || name == "os_name"
            || name == "os_version"
        {
            SysLib::call_function(name, args, exec_allowed, effective_permission_checker)
        } else if name == "now"
            || (name == "sleep" && args.len() == 1)
            || (name == "async_sleep" && args.len() == 1)
        {
            TimeLib::call_function(name, args, time_override)
        } else if name.starts_with("json_") {
            JsonLib::call_function(name, args)
        } else if name.starts_with("regex_") {
            RegexLib::call_function(name, args)
        } else if name.starts_with("path_") {
            PathLib::call_function(name, args)
        } else if name.starts_with("log_") || name == "log" {
            LogLib::call_function(name, args)
        } else if name.starts_with("url_")
            || name.starts_with("encode_uri")
            || name.starts_with("decode_uri")
        {
            UrlLib::call_function(name, args)
        } else if name == "format_time"
            || name == "time_format"
            || name == "parse_time"
            || name == "time_parse"
            || name == "now_utc"
            || name == "now_local"
            || name == "parse_datetime"
            || name == "format_datetime"
            || name == "datetime_add"
            || name == "datetime_diff"
        {
            TimeLib::call_function(name, args, time_override)
        } else if name == "cli_parse" || name == "proc_run" || name == "proc_pipe" {
            SysLib::call_function(name, args, exec_allowed, effective_permission_checker)
        } else if name == "chdir"
            || name == "pid"
            || name == "user"
            || name == "home"
            || name == "uid"
            || name == "gid"
            || name == "spawn"
            || name == "kill"
            || name == "wait"
        {
            SysLib::call_function(name, args, exec_allowed, effective_permission_checker)
        } else if name.starts_with("assert") || name.starts_with("test_") || name == "expect_error" {
            TestLib::call_function(name, args)
        } else if name == "tool_exec" || name == "tool_list" || name == "tool_info" {
            // Tool execution functions.
            // audit_trail and policy are not accessible here (borrow conflict with executor);
            // the VM intercepts tool functions before reaching this path for audit/policy wiring.
            // The degraded path in ToolLib handles the None audit_trail gracefully.
            ToolLib::call_function(name, args, effective_permission_checker, None, None, None)
        } else if name == "grant_capability"
            || name == "use_capability"
            || name == "revoke_capability"
            || name == "capability_valid"
        {
            // Capability functions require VM executor that implements CapabilityExecutor
            // This is handled by the VM directly calling the capability methods
            // For now, return an error indicating these functions need VM integration
            Err(crate::runtime::RuntimeError::new(format!(
                "Capability function '{}' requires VM executor. Use grant_capability() etc. from VM context.",
                name
            )))
        } else if name == "async_cancel_token" {
            // Group 26.3: create a new cancellation token; returns integer ID.
            let id = {
                let mut next = CANCEL_TOKEN_NEXT_ID.lock().unwrap();
                let id = *next;
                *next += 1;
                id
            };
            CANCEL_TOKENS.lock().unwrap().insert(
                id,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            );
            Ok(crate::runtime::Value::Integer(id))
        } else if name == "async_cancel" {
            // Group 26.3: cancel a token by ID; returns null.
            match args.first() {
                Some(crate::runtime::Value::Integer(id)) => {
                    if let Some(flag) =
                        CANCEL_TOKENS.lock().unwrap().get(id)
                    {
                        flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    Ok(crate::runtime::Value::Null)
                }
                _ => Err(crate::runtime::RuntimeError::new(
                    "async_cancel(token_id): expected integer token ID".to_string(),
                )),
            }
        } else if name == "is_cancelled" {
            // Group 26.3: check whether a token has been cancelled.
            match args.first() {
                Some(crate::runtime::Value::Integer(id)) => {
                    let cancelled = CANCEL_TOKENS
                        .lock()
                        .unwrap()
                        .get(id)
                        .map(|f| f.load(std::sync::atomic::Ordering::Relaxed))
                        .unwrap_or(false);
                    Ok(crate::runtime::Value::Boolean(cancelled))
                }
                _ => Err(crate::runtime::RuntimeError::new(
                    "is_cancelled(token_id): expected integer token ID".to_string(),
                )),
            }
        } else if name == "async_cancel_token_drop" {
            // Group 26.3: release a cancellation token.
            if let Some(crate::runtime::Value::Integer(id)) = args.first() {
                CANCEL_TOKENS.lock().unwrap().remove(id);
            }
            Ok(crate::runtime::Value::Null)
        } else if name == "await_all" || name == "await_any" {
            // Task 12.1: concurrent future combinators
            let futures: Vec<_> = match args.first() {
                Some(crate::runtime::Value::Array(arr)) => arr.clone(),
                _ => return Err(crate::runtime::RuntimeError::new(format!(
                    "{} expects an array of Future values", name
                ))),
            };
            if name == "await_all" {
                // Block on all futures and collect results in order
                let mut results = Vec::with_capacity(futures.len());
                for fut in futures {
                    match fut {
                        crate::runtime::Value::Future(handle) => {
                            let v = handle.resolve().map_err(|e| crate::runtime::RuntimeError::new(e))?;
                            results.push(v);
                        }
                        other => results.push(other), // non-future: pass through
                    }
                }
                Ok(crate::runtime::Value::Array(results))
            } else {
                // await_any: return first future to resolve
                // Naive implementation: resolve in order, return first Ok
                for fut in futures {
                    match fut {
                        crate::runtime::Value::Future(handle) => {
                            let v = handle.resolve().map_err(|e| crate::runtime::RuntimeError::new(e))?;
                            return Ok(v);
                        }
                        other => return Ok(other),
                    }
                }
                Ok(crate::runtime::Value::Null)
            }
        } else if name.starts_with("gzip_") {
            BytesLib::call_function(name, args)
        } else if name.starts_with("bytes_") {
            BytesLib::call_function(name, args)
        } else if name == "template_render" {
            TemplateLib::call_function(name, args)
        } else if name == "db_open"
            || name == "db_exec"
            || name == "db_close"
            || name == "db_connect"
            || name == "db_query"
            || name == "db_execute"
            || name == "db_transaction"
        {
            DbLib::call_function(name, args, effective_permission_checker)
        } else if name == "ffi_load" || name == "ffi_call" || name == "ffi_close" {
            #[cfg(feature = "ffi")]
            return FfiLib::call_function(name, args, effective_permission_checker);
            #[cfg(not(feature = "ffi"))]
            return Err(crate::runtime::RuntimeError::new(format!(
                "FFI function '{}' requires the 'ffi' feature. \
                 Rebuild with: cargo build --features ffi",
                name
            )))
        } else if name == "plugin_load" || name == "plugin_functions" || name == "plugin_call" {
            PluginLib::call_function(name, args, effective_permission_checker)
        } else if name == "wasm_load" || name == "wasm_call" || name == "wasm_close" {
            crate::stdlib::wasm_exec::call_function(name, args)
        } else if matches!(name,
            "FileNotFoundError" | "PermissionError" | "NetworkError" |
            "ParseError" | "TypeError" | "ValueError" |
            "IndexError" | "TimeoutError"
        ) {
            errors::ErrorLib::call_function(name, args)
        } else {
            Err(crate::runtime::RuntimeError::new(format!("Unknown standard library function: {}", name))
                .with_hint("Check the function name spelling. Use standard library functions like print, len, array_map, etc.".to_string()))
        }
    }
}
