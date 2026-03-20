#[cfg(feature = "bytecode")]
use txtcode::compiler::bytecode::BytecodeCompiler;
#[cfg(feature = "bytecode")]
use txtcode::lexer::Lexer;
#[cfg(feature = "bytecode")]
use txtcode::parser::Parser;
#[cfg(feature = "bytecode")]
use txtcode::tools::debugger::Debugger;

#[cfg(feature = "bytecode")]
fn compile_src(source: &str) -> txtcode::compiler::bytecode::Bytecode {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut compiler = BytecodeCompiler::new();
    compiler.compile(&program)
}

// Task 11.2 — debug_info is populated with (ip, line) entries
#[cfg(feature = "bytecode")]
#[test]
fn test_debug_info_is_populated() {
    // Three assignments on separate lines → at least 3 debug_info entries
    let src = "store → a → 1\nstore → b → 2\nstore → c → 3";
    let bytecode = compile_src(src);
    assert!(
        !bytecode.debug_info.is_empty(),
        "debug_info should be populated"
    );
    // Lines should include 1, 2, 3
    let lines: Vec<usize> = bytecode.debug_info.iter().map(|&(_, l)| l).collect();
    assert!(lines.contains(&1), "line 1 expected in debug_info");
    assert!(lines.contains(&2), "line 2 expected in debug_info");
    assert!(lines.contains(&3), "line 3 expected in debug_info");
}

// Task 11.2 — add_breakpoint_at_line resolves to an instruction index
#[cfg(feature = "bytecode")]
#[test]
fn test_add_breakpoint_at_line() {
    let src = "store → a → 1\nstore → b → 2\nstore → c → 3";
    let bytecode = compile_src(src);
    let mut debugger = Debugger::new();
    debugger.load(bytecode);

    // Set breakpoint at line 2 — should succeed and return an ip
    let result = debugger.add_breakpoint_at_line(2);
    assert!(result.is_some(), "should resolve line 2 to an ip");

    // Breakpoints list should contain that ip
    let bps = debugger.list_breakpoints();
    assert!(!bps.is_empty(), "breakpoints list should not be empty after adding");
}

// Task 11.2 — source_line_for_ip returns the correct line
#[cfg(feature = "bytecode")]
#[test]
fn test_source_line_for_ip() {
    let src = "store → a → 1\nstore → b → 2\nstore → c → 3";
    let bytecode = compile_src(src);
    let mut debugger = Debugger::new();
    debugger.load(bytecode);

    // ip=0 should map to line 1
    let line = debugger.source_line_for_ip(0);
    assert_eq!(line, Some(1), "ip=0 should map to line 1");
}

// Task 11.2 — step advances one instruction at a time
#[cfg(feature = "bytecode")]
#[test]
fn test_step_advances() {
    let src = "store → x → 42";
    let bytecode = compile_src(src);
    let mut debugger = Debugger::new();
    debugger.load(bytecode);

    let state = debugger.step().expect("step should succeed");
    assert_eq!(state.ip, 0, "first step should be at ip=0");
    assert!(!state.done, "should not be done after first step");
}
