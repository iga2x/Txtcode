/// Unit tests for Task 29.2 — WASM Binary Output

#[cfg(feature = "wasm")]
mod wasm_tests {
    use txtcode::compiler::bytecode::{Bytecode, BytecodeCompiler, Constant, Instruction};
    use txtcode::compiler::wasm_binary::compile_to_binary;
    use txtcode::lexer::Lexer;
    use txtcode::parser::Parser;

    fn compile_source(src: &str) -> Bytecode {
        let mut lexer = Lexer::new(src.to_string());
        let tokens = lexer.tokenize().expect("lex error");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse error");
        let mut compiler = BytecodeCompiler::new();
        compiler.compile(&program)
    }

    // ── Test 1: binary is valid WASM ─────────────────────────────────────────

    #[test]
    fn test_wasm_binary_starts_with_magic() {
        // A minimal bytecode: push 42 and halt
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0),
                Instruction::ReturnValue,
            ],
            constants: vec![Constant::Integer(42)],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("compile_to_binary failed");
        // WASM magic: 0x00 0x61 0x73 0x6d
        assert!(bytes.starts_with(b"\0asm"), "output does not start with WASM magic");
    }

    // ── Test 2: wasmparser can parse the output ───────────────────────────────

    #[test]
    fn test_wasm_binary_parseable_by_wasmparser() {
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0),
                Instruction::PushConstant(1),
                Instruction::Add,
                Instruction::ReturnValue,
            ],
            constants: vec![Constant::Integer(3), Constant::Integer(4)],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("compile_to_binary failed");

        // wasmparser — iterate all payloads without error
        use wasmparser::{Parser as WasmParser, Payload};
        let mut found_end = false;
        for payload in WasmParser::new(0).parse_all(&bytes) {
            match payload.expect("wasmparser error") {
                Payload::End(_) => {
                    found_end = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(found_end, "wasmparser did not reach End payload");
    }

    // ── Test 3: integer arithmetic program produces correct bytes ─────────────

    #[test]
    fn test_wasm_binary_arithmetic_program() {
        // "store → x → 10\nstore → y → 32\nx + y" should compile fine
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0), // 10
                Instruction::StoreVar("x".to_string()),
                Instruction::PushConstant(1), // 32
                Instruction::StoreVar("y".to_string()),
                Instruction::LoadVar("x".to_string()),
                Instruction::LoadVar("y".to_string()),
                Instruction::Add,
                Instruction::ReturnValue,
            ],
            constants: vec![Constant::Integer(10), Constant::Integer(32)],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("compile_to_binary failed");
        // Ensure non-trivial output size (type + function + export + code sections)
        assert!(
            bytes.len() > 20,
            "binary too small ({}), expected >20 bytes",
            bytes.len()
        );
        // Starts with WASM magic and version
        assert!(bytes.starts_with(b"\0asm\x01\x00\x00\x00"));
    }

    // ── H.1: String constant tests ───────────────────────────────────────────

    /// H.1 Test 1: single string constant compiles to valid WASM with data section
    #[test]
    fn test_h1_string_constant_compiles() {
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0),
                Instruction::ReturnValue,
            ],
            constants: vec![Constant::String("hello".to_string())],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("string constant should compile");
        assert!(bytes.starts_with(b"\0asm\x01\x00\x00\x00"), "missing WASM magic");
        // Data section is present — search for "hello" bytes in the binary
        let needle = b"hello";
        assert!(
            bytes.windows(needle.len()).any(|w| w == needle),
            "string bytes 'hello' not found in WASM binary"
        );
    }

    /// H.1 Test 2: multiple distinct strings are all present in the binary
    #[test]
    fn test_h1_multiple_strings() {
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0),
                Instruction::Pop,
                Instruction::PushConstant(1),
                Instruction::ReturnValue,
            ],
            constants: vec![
                Constant::String("foo".to_string()),
                Constant::String("bar".to_string()),
            ],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("multiple strings should compile");
        assert!(bytes.windows(3).any(|w| w == b"foo"), "'foo' not in binary");
        assert!(bytes.windows(3).any(|w| w == b"bar"), "'bar' not in binary");
    }

    /// H.1 Test 3: empty string compiles without error
    #[test]
    fn test_h1_empty_string() {
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0),
                Instruction::ReturnValue,
            ],
            constants: vec![Constant::String("".to_string())],
            debug_info: vec![],
        };
        // Empty string → pool is empty → no memory/data section needed
        let bytes = compile_to_binary(&bytecode).expect("empty string should compile");
        assert!(bytes.starts_with(b"\0asm"), "missing WASM magic for empty string");
    }

    /// H.1 Test 4: same string used twice → interned once (pool dedup)
    #[test]
    fn test_h1_string_reuse_interned_once() {
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0),
                Instruction::Pop,
                Instruction::PushConstant(1), // same string again
                Instruction::ReturnValue,
            ],
            constants: vec![
                Constant::String("repeat".to_string()),
                Constant::String("repeat".to_string()),
            ],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("reused string should compile");
        // Count occurrences of "repeat" in binary — should appear exactly once in the data segment
        let needle = b"repeat";
        let count = bytes.windows(needle.len()).filter(|w| *w == needle).count();
        assert_eq!(count, 1, "interned string 'repeat' should appear exactly once in binary, got {}", count);
    }

    /// H.1 Test 5: string alongside integers compiles correctly
    #[test]
    fn test_h1_string_with_integers() {
        let bytecode = Bytecode {
            instructions: vec![
                Instruction::PushConstant(0), // integer
                Instruction::PushConstant(1), // string
                Instruction::Pop,
                Instruction::ReturnValue,
            ],
            constants: vec![
                Constant::Integer(99),
                Constant::String("world".to_string()),
            ],
            debug_info: vec![],
        };
        let bytes = compile_to_binary(&bytecode).expect("string+integer should compile");
        assert!(bytes.starts_with(b"\0asm"), "missing WASM magic");
        assert!(bytes.windows(5).any(|w| w == b"world"), "'world' not in binary");
    }

    // ── Fallback when feature is absent ──────────────────────────────────────
    // (This test only runs without the wasm feature — verified by build matrix)
}

// Always-present test: non-wasm stub returns Err for any input
#[cfg(not(feature = "wasm"))]
#[test]
fn test_wasm_binary_stub_returns_error() {
    use txtcode::compiler::wasm_binary::compile_to_binary;
    // The stub is generic over T — pass a dummy value
    let dummy: u8 = 0;
    assert!(compile_to_binary(&dummy).is_err());
}
