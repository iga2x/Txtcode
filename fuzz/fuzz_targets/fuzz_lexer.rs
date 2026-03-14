#![no_main]
// Fuzz target: Lexer::tokenize — a malformed .tc source must never panic.
use libfuzzer_sys::fuzz_target;
use txtcode::lexer::Lexer;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(s.to_string());
        // Any result (Ok or Err) is acceptable — only panics are bugs.
        let _ = lexer.tokenize();
    }
});
