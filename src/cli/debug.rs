//! `txtcode debug` — interactive bytecode debugger.

use crate::compiler::bytecode::BytecodeCompiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::tools::debugger::Debugger;
use crate::validator::Validator;
use std::fs;
use std::path::PathBuf;

pub fn start_debug_repl(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::DefaultEditor;

    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }

    let source = fs::read_to_string(file)?;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    Validator::validate_program(&program).map_err(|e| format!("Validation error: {}", e))?;
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let total = bytecode.instructions.len();
    let mut debugger = Debugger::new();
    debugger.load(bytecode);

    println!(
        "Txt-code Debugger — {} ({} instructions)",
        file.display(),
        total
    );
    println!(
        "Commands: step/s, continue/c, break/b <n>, inspect/i <var>, stack, vars, quit/q, help"
    );
    println!("ip=0 ready");

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("(debug) ");
        let line = match readline {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(trimmed);
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        match parts[0] {
            "step" | "s" => match debugger.step() {
                Ok(state) => {
                    if state.done {
                        println!("Program finished.");
                    } else {
                        println!("ip={} | {}", state.ip, state.instruction);
                    }
                }
                Err(e) => eprintln!("Step error: {}", e),
            },
            "continue" | "c" => match debugger.continue_execution() {
                Ok(_) => println!("Execution complete."),
                Err(e) => println!("{}", e),
            },
            "break" | "b" => {
                if let Some(addr_str) = parts.get(1) {
                    if let Ok(addr) = addr_str.trim().parse::<usize>() {
                        debugger.add_breakpoint(addr);
                        println!("Breakpoint set at ip={}", addr);
                    } else {
                        eprintln!("Usage: break <instruction_index>");
                    }
                } else {
                    let bps = debugger.list_breakpoints();
                    if bps.is_empty() {
                        println!("No breakpoints set.");
                    } else {
                        println!("Breakpoints: {:?}", bps);
                    }
                }
            }
            "inspect" | "i" => {
                if let Some(name) = parts.get(1) {
                    match debugger.inspect_variable(name.trim()) {
                        Some(val) => println!("{} = {:?}", name.trim(), val),
                        None => println!("Variable '{}' not found", name.trim()),
                    }
                } else {
                    eprintln!("Usage: inspect <variable>");
                }
            }
            "stack" => {
                let stack = debugger.get_stack();
                if stack.is_empty() {
                    println!("Stack: (empty)");
                } else {
                    println!("Stack ({} items):", stack.len());
                    for (i, val) in stack.iter().enumerate().rev() {
                        println!("  [{}] {:?}", i, val);
                    }
                }
            }
            "vars" => {
                let vars = debugger.get_all_variables();
                if vars.is_empty() {
                    println!("No variables defined.");
                } else {
                    println!("Variables ({}):", vars.len());
                    for (k, v) in &vars {
                        println!("  {} = {:?}", k, v);
                    }
                }
            }
            "callstack" => {
                let frames = debugger.get_call_stack();
                if frames.is_empty() {
                    println!("Call stack: (empty)");
                } else {
                    for frame in &frames {
                        println!("  {}", frame);
                    }
                }
            }
            "help" | "?" => {
                println!("Commands:");
                println!("  step / s               — execute one instruction");
                println!("  continue / c           — run until breakpoint or end");
                println!("  break / b <n>          — set breakpoint at instruction n");
                println!("  break / b              — list breakpoints");
                println!("  inspect / i <var>      — inspect variable value");
                println!("  stack                  — show operand stack");
                println!("  vars                   — show all variables");
                println!("  callstack              — show call stack frames");
                println!("  quit / q               — exit debugger");
            }
            "quit" | "q" => break,
            _ => eprintln!("Unknown command '{}'. Type 'help' for commands.", parts[0]),
        }
    }

    Ok(())
}
