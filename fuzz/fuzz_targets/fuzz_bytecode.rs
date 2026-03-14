#![no_main]
// Fuzz target: BytecodeCompiler::compile — a valid AST must never cause a panic.
use libfuzzer_sys::fuzz_target;
use txtcode::compiler::bytecode::BytecodeCompiler;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(s.to_string());
        if let Ok(tokens) = lexer.tokenize() {
            let mut parser = Parser::new(tokens);
            if let Ok(program) = parser.parse() {
                let mut compiler = BytecodeCompiler::new();
                let _ = compiler.compile(&program);
            }
        }
    }
});
