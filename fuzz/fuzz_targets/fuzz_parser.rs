#![no_main]
// Fuzz target: Parser::parse — a malformed token stream must never panic.
use libfuzzer_sys::fuzz_target;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(s.to_string());
        if let Ok(tokens) = lexer.tokenize() {
            let mut parser = Parser::new(tokens);
            let _ = parser.parse();
        }
    }
});
