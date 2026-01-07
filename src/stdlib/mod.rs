pub mod core;
pub mod crypto;
pub mod net;
pub mod io;
pub mod sys;

pub use core::CoreLib;
pub use crypto::CryptoLib;
pub use net::NetLib;
pub use io::IOLib;
pub use sys::SysLib;

/// Standard library function dispatcher
pub struct StdLib;

impl StdLib {
    /// Call any standard library function
    pub fn call_function(name: &str, args: &[crate::runtime::vm::Value]) -> Result<crate::runtime::vm::Value, crate::runtime::vm::RuntimeError> {
        // Route to appropriate library
        if name.starts_with("str_") || name.starts_with("math_") || name.starts_with("array_") || 
           name == "len" || name == "type" || name.starts_with("to_") ||
           name == "string" || name == "int" || name == "float" || name == "bool" ||
           name == "input" {
            CoreLib::call_function(name, args)
        } else if name.starts_with("sha") || name.starts_with("random") || name == "encrypt" || name == "decrypt" {
            CryptoLib::call_function(name, args)
        } else if name.starts_with("http") || name.starts_with("tcp") {
            NetLib::call_function(name, args)
        } else if name.starts_with("read") || name.starts_with("write") || name.starts_with("file") || name.starts_with("list") {
            IOLib::call_function(name, args)
        } else if name == "getenv" || name == "setenv" || name == "platform" || name == "arch" || name == "exec" {
            SysLib::call_function(name, args)
        } else {
            Err(crate::runtime::vm::RuntimeError {
                message: format!("Unknown standard library function: {}", name),
            })
        }
    }
}

