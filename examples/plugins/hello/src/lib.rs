//! Hello-world Txtcode plugin.
//!
//! Build:
//!   cargo build --release
//!   # output: target/release/libtxtcode_hello_plugin.so (Linux)
//!   #         target/release/libtxtcode_hello_plugin.dylib (macOS)
//!
//! Use from Txtcode (requires --allow-ffi=PATH):
//!   store → plugin → plugin_load("libtxtcode_hello_plugin.so")
//!   store → funcs → plugin_functions("libtxtcode_hello_plugin.so")
//!   store → result → plugin_call("libtxtcode_hello_plugin.so", "hello_from_plugin", [])
//!   println(result)   // prints 42

use txtcode_plugin_sdk::txtcode_plugin;

// Declare the plugin — generates the three required C-ABI entry points.
txtcode_plugin! {
    name: "hello",
    functions: [hello_from_plugin, add_numbers],
}

/// Returns 42 — the answer to everything.
fn hello_from_plugin(_args: &[i64]) -> i64 {
    42
}

/// Adds two numbers: add_numbers([a, b]) → a + b
fn add_numbers(args: &[i64]) -> i64 {
    let a = args.first().copied().unwrap_or(0);
    let b = args.get(1).copied().unwrap_or(0);
    a + b
}
