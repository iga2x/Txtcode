/// Task 28.3 — Web REPL Playground entry point
///
/// This binary is compiled to `wasm32-unknown-unknown` and loaded by the
/// browser playground. The single exported function `eval_script` takes a
/// Txt-code source string and returns the result as a JSON string.
///
/// Build:
/// ```text
/// cargo build --target wasm32-unknown-unknown --features wasm --bin playground
/// ```

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Evaluate a Txt-code source string and return the result as JSON.
///
/// On success: `{"ok": "<result>"}`
/// On error:   `{"error": "<message>"}`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eval_script(source: &str) -> String {
    use txtcode::embed::TxtcodeEngine;
    let mut engine = TxtcodeEngine::new();
    match engine.eval(source) {
        Ok(val) => format!(r#"{{"ok":"{}"}}"#, escape_json(&format!("{:?}", val))),
        Err(e) => format!(r#"{{"error":"{}"}}"#, escape_json(&e.message())),
    }
}

#[cfg(target_arch = "wasm32")]
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

// Non-WASM build: no-op main so `cargo build` doesn't fail on native targets.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("This binary is intended for `wasm32-unknown-unknown` target only.");
    eprintln!("Build with: cargo build --target wasm32-unknown-unknown --features wasm --bin playground");
}
