//! # Txtcode Plugin SDK
//!
//! Provides the macros and types needed to write a native Txtcode plugin.
//!
//! ## Quick start
//!
//! 1. Add this to your plugin's `Cargo.toml`:
//!    ```toml
//!    [lib]
//!    crate-type = ["cdylib"]
//!
//!    [dependencies]
//!    txtcode-plugin-sdk = { path = "../../crates/plugin-sdk" }
//!    ```
//!
//! 2. Use the `txtcode_plugin!` macro:
//!    ```rust,ignore
//!    use txtcode_plugin_sdk::txtcode_plugin;
//!
//!    txtcode_plugin! {
//!        name: "hello",
//!        functions: [hello_from_plugin, add_numbers],
//!    }
//!
//!    fn hello_from_plugin(_args: &[i64]) -> i64 { 42 }
//!    fn add_numbers(args: &[i64]) -> i64 { args.get(0).copied().unwrap_or(0) + args.get(1).copied().unwrap_or(0) }
//!    ```
//!
//! ## Plugin ABI contract
//!
//! The compiled `.so`/`.dylib` must export:
//! - `txtcode_plugin_name() -> *const c_char` — null-terminated plugin name
//! - `txtcode_functions() -> *const *const c_char` — null-terminated array of function names
//! - `txtcode_call(fn_name, argc, argv) -> i64` — dispatch a function call

/// Declare a Txtcode-compatible plugin.
///
/// Generates the three required C-ABI entry points:
/// - `txtcode_plugin_name`
/// - `txtcode_functions`
/// - `txtcode_call`
///
/// # Example
/// ```rust,ignore
/// txtcode_plugin! {
///     name: "my_plugin",
///     functions: [my_fn, another_fn],
/// }
/// fn my_fn(_args: &[i64]) -> i64 { 99 }
/// fn another_fn(args: &[i64]) -> i64 { args.iter().sum() }
/// ```
#[macro_export]
macro_rules! txtcode_plugin {
    (
        name: $plugin_name:expr,
        functions: [$($fn_name:ident),* $(,)?],
    ) => {
        /// Null-terminated plugin name string (static storage).
        static PLUGIN_NAME: &[u8] = concat!($plugin_name, "\0").as_bytes();

        /// Null-terminated array of null-terminated function name strings.
        static FUNCTION_NAMES_RAW: &[*const std::os::raw::c_char] = {
            // Build a static slice of pointers; each entry points into a literal.
            // SAFETY: all literals are 'static.
            &[
                $(concat!(stringify!($fn_name), "\0").as_ptr() as *const std::os::raw::c_char,)*
                std::ptr::null(),
            ]
        };

        /// Return the plugin name as a C string.
        #[no_mangle]
        pub extern "C" fn txtcode_plugin_name() -> *const std::os::raw::c_char {
            PLUGIN_NAME.as_ptr() as *const std::os::raw::c_char
        }

        /// Return a null-terminated array of exported function name C strings.
        #[no_mangle]
        pub extern "C" fn txtcode_functions() -> *const *const std::os::raw::c_char {
            FUNCTION_NAMES_RAW.as_ptr()
        }

        /// Dispatch a function call by name.
        ///
        /// # Safety
        /// `fn_name` must be a valid null-terminated C string.
        /// `argv` must point to at least `argc` i64 values (or be null when argc == 0).
        #[no_mangle]
        pub unsafe extern "C" fn txtcode_call(
            fn_name: *const std::os::raw::c_char,
            argc: i32,
            argv: *const i64,
        ) -> i64 {
            let name = if fn_name.is_null() { return -1; }
                else { std::ffi::CStr::from_ptr(fn_name).to_string_lossy() };
            let args: &[i64] = if argc > 0 && !argv.is_null() {
                std::slice::from_raw_parts(argv, argc as usize)
            } else {
                &[]
            };
            match name.as_ref() {
                $(stringify!($fn_name) => $fn_name(args),)*
                _ => -1,
            }
        }
    };
}
