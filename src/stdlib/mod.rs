pub mod bytes;
pub mod capabilities;
pub mod core;
pub mod crypto;
pub mod ffi;
pub mod function_executor;
pub mod io;
pub mod json;
pub mod log;
#[cfg(feature = "net")]
pub mod net;
pub mod path;
pub mod permission_checker;
pub mod regex;
pub mod sys;
pub mod test;
pub mod time;
pub mod tools;
pub mod url;

pub use bytes::BytesLib;
pub use capabilities::{CapabilityExecutor, CapabilityLib};
pub use core::CoreLib;
pub use crypto::CryptoLib;
pub use ffi::FfiLib;
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
            || name.starts_with("yaml_")
            || name == "math_random_float"
            || name == "format"
        {
            CoreLib::call_function(name, args, executor)
        } else if name.starts_with("sha")
            || name.starts_with("crypto_random")
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
        {
            CryptoLib::call_function(name, args, seed_override)
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
        } else if name.starts_with("http")
            || name.starts_with("tcp")
            || name == "udp_send"
            || name == "resolve"
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
        } else if name == "now" || (name == "sleep" && args.len() == 1) {
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
        } else if name.starts_with("assert") || name.starts_with("test_") {
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
        } else if name.starts_with("bytes_") {
            BytesLib::call_function(name, args)
        } else if name == "ffi_load" || name == "ffi_call" || name == "ffi_close" {
            #[cfg(feature = "ffi")]
            return FfiLib::call_function(name, args, effective_permission_checker);
            #[cfg(not(feature = "ffi"))]
            return Err(crate::runtime::RuntimeError::new(format!(
                "FFI function '{}' requires the 'ffi' feature. \
                 Rebuild with: cargo build --features ffi",
                name
            )))
        } else {
            Err(crate::runtime::RuntimeError::new(format!("Unknown standard library function: {}", name))
                .with_hint("Check the function name spelling. Use standard library functions like print, len, array_map, etc.".to_string()))
        }
    }
}
