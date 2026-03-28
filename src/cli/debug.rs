//! `txtcode debug` — interactive bytecode debugger.

use crate::compiler::bytecode::BytecodeCompiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::tools::debugger::Debugger;
use crate::validator::Validator;
use std::fs;
use std::path::PathBuf;

/// Print up to 3 lines of source context centred on `target_line`, with a `→` marker.
fn print_source_context(source_lines: &[&str], target_line: usize) {
    let start = target_line.saturating_sub(1);
    let end = (target_line + 1).min(source_lines.len());
    for ln in start..=end {
        if ln == 0 || ln > source_lines.len() {
            continue;
        }
        let marker = if ln == target_line { "→" } else { " " };
        println!("  {} {:4} │ {}", marker, ln, source_lines[ln - 1]);
    }
}

pub fn start_debug_repl(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::DefaultEditor;

    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }

    let source = fs::read_to_string(file)?;
    let source_lines: Vec<&str> = source.lines().collect();

    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    Validator::validate_program(&program).map_err(|e| format!("Validation error: {}", e))?;
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let total = bytecode.instructions.len();
    let has_debug_info = !bytecode.debug_info.is_empty();
    let mut debugger = Debugger::new();
    debugger.load(bytecode);

    println!(
        "Txt-code Debugger — {} ({} instructions)",
        file.display(),
        total
    );
    if !has_debug_info {
        println!("(no debug symbols — source line mapping unavailable)");
    }
    println!(
        "Commands: step/s, next/n, continue/c, break/b <line>, print/p <var>, stack, vars, run, quit/q, help"
    );
    println!("Type 'run' to run to end, or 'break <line>' then 'run' to stop at a line.");

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("(txtcode-dbg) ");
        let input = match readline {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(trimmed);
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();

        match parts[0] {
            // ── step ──────────────────────────────────────────────────────
            "step" | "s" => match debugger.step() {
                Ok(state) => {
                    if state.done {
                        println!("Program finished.");
                    } else {
                        let src_line = debugger.source_line_for_ip(state.ip);
                        if let Some(ln) = src_line {
                            println!("ip={} line={} | {}", state.ip, ln, state.instruction);
                            print_source_context(&source_lines, ln);
                        } else {
                            println!("ip={} | {}", state.ip, state.instruction);
                        }
                    }
                }
                Err(e) => eprintln!("Step error: {}", e),
            },

            // ── next (step-over) ──────────────────────────────────────────
            "next" | "n" => match debugger.step_over() {
                Ok(state) => {
                    if state.done {
                        println!("Program finished.");
                    } else {
                        let src_line = debugger.source_line_for_ip(state.ip);
                        if let Some(ln) = src_line {
                            println!("ip={} line={} | {}", state.ip, ln, state.instruction);
                            print_source_context(&source_lines, ln);
                        } else {
                            println!("ip={} | {}", state.ip, state.instruction);
                        }
                    }
                }
                Err(e) => eprintln!("Next error: {}", e),
            },

            // ── continue / run ────────────────────────────────────────────
            "continue" | "c" | "run" => match debugger.continue_execution() {
                Ok(_) => println!("Execution complete."),
                Err(e) => {
                    // A breakpoint hit returns an Err with info
                    let ip = debugger.current_ip();
                    let src_line = debugger.source_line_for_ip(ip);
                    if let Some(ln) = src_line {
                        println!("Stopped at line {} (ip={}): {}", ln, ip, e);
                        print_source_context(&source_lines, ln);
                    } else {
                        println!("{}", e);
                    }
                }
            },

            // ── break ─────────────────────────────────────────────────────
            "break" | "b" => {
                if let Some(arg) = parts.get(1) {
                    let arg = arg.trim();
                    if let Ok(line_num) = arg.parse::<usize>() {
                        // Try source line first; fall back to raw ip if no debug info
                        match debugger.add_breakpoint_at_line(line_num) {
                            Some(ip) => println!("Breakpoint set at line {} (ip={})", line_num, ip),
                            None => {
                                // No debug info: treat as instruction index
                                debugger.add_breakpoint(line_num);
                                println!("Breakpoint set at ip={} (no line mapping)", line_num);
                            }
                        }
                    } else {
                        eprintln!("Usage: break <line_number>");
                    }
                } else {
                    // List breakpoints
                    let bps = debugger.list_breakpoints();
                    if bps.is_empty() {
                        println!("No breakpoints set.");
                    } else {
                        for &ip in bps {
                            if let Some(ln) = debugger.source_line_for_ip(ip) {
                                println!("  ip={} (line {})", ip, ln);
                            } else {
                                println!("  ip={}", ip);
                            }
                        }
                    }
                }
            }

            // ── print / inspect ───────────────────────────────────────────
            "print" | "p" | "inspect" | "i" => {
                if let Some(name) = parts.get(1) {
                    match debugger.inspect_variable(name.trim()) {
                        Some(val) => println!("{} = {:?}", name.trim(), val),
                        None => println!("Variable '{}' not found in scope.", name.trim()),
                    }
                } else {
                    eprintln!("Usage: print <variable>");
                }
            }

            // ── stack ─────────────────────────────────────────────────────
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

            // ── vars ──────────────────────────────────────────────────────
            "vars" => {
                let vars = debugger.get_all_variables();
                if vars.is_empty() {
                    println!("No variables defined.");
                } else {
                    println!("Variables ({}):", vars.len());
                    let mut sorted: Vec<_> = vars.iter().collect();
                    sorted.sort_by_key(|(k, _)| k.as_str());
                    for (k, v) in sorted {
                        println!("  {} = {:?}", k, v);
                    }
                }
            }

            // ── callstack ─────────────────────────────────────────────────
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

            // ── where ─────────────────────────────────────────────────────
            "where" | "w" => {
                let ip = debugger.current_ip();
                if let Some(ln) = debugger.source_line_for_ip(ip) {
                    println!("Paused at line {} (ip={})", ln, ip);
                    print_source_context(&source_lines, ln);
                } else {
                    println!("Paused at ip={} (no source mapping)", ip);
                }
            }

            // ── help ──────────────────────────────────────────────────────
            "help" | "?" => {
                println!("Commands:");
                println!("  step / s               — execute one instruction");
                println!("  next / n               — step to next source line (step over)");
                println!("  continue / c / run     — run until breakpoint or end");
                println!("  break / b <line>       — set breakpoint at source line");
                println!("  break / b              — list breakpoints");
                println!("  print / p <var>        — print variable value");
                println!("  vars                   — show all variables in scope");
                println!("  stack                  — show operand stack");
                println!("  callstack              — show call stack frames");
                println!("  where / w              — show current source position");
                println!("  quit / q               — exit debugger");
            }

            "quit" | "q" => break,

            _ => eprintln!("Unknown command '{}'. Type 'help' for commands.", parts[0]),
        }
    }

    Ok(())
}
