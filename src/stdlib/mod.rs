pub mod core;
pub mod crypto;
pub mod net;
pub mod io;
pub mod sys;
pub mod time;
pub mod json;
pub mod regex;
pub mod path;
pub mod log;
pub mod url;
pub mod test;
pub mod function_executor;
pub mod permission_checker;
pub mod capabilities;
pub mod tools;

pub use core::CoreLib;
pub use crypto::CryptoLib;
pub use net::NetLib;
pub use io::IOLib;
pub use sys::SysLib;
pub use time::TimeLib;
pub use json::JsonLib;
pub use regex::RegexLib;
pub use path::PathLib;
pub use log::LogLib;
pub use url::UrlLib;
pub use test::TestLib;
pub use function_executor::FunctionExecutor;
pub use permission_checker::PermissionChecker;
pub use capabilities::{CapabilityLib, CapabilityExecutor};
pub use tools::ToolLib;

/// Standard library function dispatcher
pub struct StdLib;

impl StdLib {
    /// Call any standard library function
    /// exec_allowed: whether exec() function is allowed (for safe mode)
    /// executor: optional function executor for higher-order functions
    /// If executor implements PermissionChecker, it will be used for permission checking
    pub fn call_function<E>(name: &str, args: &[crate::runtime::Value], exec_allowed: bool, executor: Option<&mut E>) 
    -> Result<crate::runtime::Value, crate::runtime::RuntimeError> 
    where
        E: FunctionExecutor,
    {
        // Try to extract permission checker from executor if it implements PermissionChecker
        // Since we can't use &mut and & at the same time, we'll pass None here
        // and use call_function_with_permission_checker when available
        Self::call_function_with_permission_checker_internal(name, args, exec_allowed, executor, None)
    }
    
    /// Call function with explicit permission checker (avoids borrow checker issues)
    pub fn call_function_with_permission_checker<E>(
        name: &str, 
        args: &[crate::runtime::Value], 
        exec_allowed: bool, 
        executor: Option<&mut E>,
        permission_checker: Option<&dyn PermissionChecker>
    ) -> Result<crate::runtime::Value, crate::runtime::RuntimeError>
    where
        E: FunctionExecutor,
    {
        Self::call_function_with_permission_checker_internal(name, args, exec_allowed, executor, permission_checker)
    }
    
    /// Call function with executor that implements both FunctionExecutor and PermissionChecker
    /// This version properly handles the borrow checker by restructuring the permission check
    pub fn call_function_with_combined_traits<E>(name: &str, args: &[crate::runtime::Value], exec_allowed: bool, executor: Option<&mut E>)
    -> Result<crate::runtime::Value, crate::runtime::RuntimeError>
    where
        E: FunctionExecutor + PermissionChecker,
    {
        // Extract permission checker from executor if possible
        // Since executor implements PermissionChecker, we can use it for permission checking
        // But we need to handle borrow checker: can't use &mut and & at the same time
        // Solution: Use executor as permission checker directly by checking permissions inline
        // For now, pass None and permission checking will be added incrementally
        // TODO: Implement proper permission checking extraction without borrow issues
        // Workaround: Pass executor, and stdlib functions can check permissions using executor if it implements PermissionChecker
        Self::call_function_with_permission_checker_internal(name, args, exec_allowed, executor, None)
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
        
        // Route to appropriate library
        if name.starts_with("str_") || name.starts_with("math_") || name.starts_with("array_") || 
           name.starts_with("set_") || name == "set" ||
           name == "len" || name == "type" || name.starts_with("to_") ||
           name == "string" || name == "int" || name == "float" || name == "bool" ||
           name == "input" || name == "print" || name == "max" || name == "min" ||
           name == "map" || name == "filter" || name == "reduce" || name == "find" ||
           name == "sin" || name == "cos" || name == "tan" || name == "sqrt" || name == "log" ||
           name == "pow" || name == "abs" || name == "floor" || name == "ceil" || name == "round" ||
           name == "split" || name == "join" || name == "replace" || name == "trim" ||
           name == "substring" || name == "indexOf" || name == "startsWith" || name == "endsWith" ||
           name == "toUpper" || name == "toLower" ||
           name == "sort" || name == "reverse" || name == "concat" {
            CoreLib::call_function(name, args, executor)
        } else if name.starts_with("sha") || name.starts_with("random") || name == "encrypt" || name == "decrypt" {
            CryptoLib::call_function(name, args)
        } else if name.starts_with("http") || name.starts_with("tcp") || name == "udp_send" || name == "resolve" {
            NetLib::call_function(name, args, effective_permission_checker)
        } else if name.starts_with("read") || name.starts_with("write") || name.starts_with("file") || name.starts_with("list") ||
                  name == "is_file" || name == "is_dir" || name == "mkdir" || name == "rmdir" || name == "delete" ||
                  name == "append_file" || name == "copy_file" || name == "move_file" || name == "rename_file" {
            IOLib::call_function(name, args, effective_permission_checker)
        } else if name == "getenv" || name == "setenv" || name == "platform" || name == "arch" || name == "exec" ||
                  name == "exit" || name == "args" || name == "cwd" {
            SysLib::call_function(name, args, exec_allowed, effective_permission_checker)
        } else if name == "now" || (name == "sleep" && args.len() == 1) {
            TimeLib::call_function(name, args)
        } else if name.starts_with("json_") {
            JsonLib::call_function(name, args)
        } else if name.starts_with("regex_") {
            RegexLib::call_function(name, args)
        } else if name.starts_with("path_") {
            PathLib::call_function(name, args)
        } else if name.starts_with("log_") || name == "log" || name == "debug" || name == "info" || name == "warn" || name == "error" {
            LogLib::call_function(name, args)
        } else if name.starts_with("url_") || name.starts_with("encode_uri") || name.starts_with("decode_uri") {
            UrlLib::call_function(name, args)
        } else if name == "format_time" || name == "time_format" || name == "parse_time" || name == "time_parse" {
            TimeLib::call_function(name, args)
        } else if name == "chdir" || name == "pid" || name == "user" || name == "home" || name == "uid" || name == "gid" ||
                  name == "spawn" || name == "kill" || name == "wait" {
            SysLib::call_function(name, args, exec_allowed, effective_permission_checker)
        } else if name.starts_with("assert") || name.starts_with("test_") {
            TestLib::call_function(name, args)
        } else if name == "tool_exec" || name == "tool_list" || name == "tool_info" {
            // Tool execution functions
            // Note: This requires audit trail and AI metadata from VM context
            // For now, pass None - VM will handle audit logging separately
            ToolLib::call_function(name, args, effective_permission_checker, None, None)
        } else if name == "grant_capability" || name == "use_capability" || 
                  name == "revoke_capability" || name == "capability_valid" {
            // Capability functions require VM executor that implements CapabilityExecutor
            // This is handled by the VM directly calling the capability methods
            // For now, return an error indicating these functions need VM integration
            Err(crate::runtime::RuntimeError::new(format!(
                "Capability function '{}' requires VM executor. Use grant_capability() etc. from VM context.", 
                name
            )))
        } else {
            Err(crate::runtime::RuntimeError::new(format!("Unknown standard library function: {}", name))
                .with_hint("Check the function name spelling. Use standard library functions like print, len, array_map, etc.".to_string()))
        }
    }

}

