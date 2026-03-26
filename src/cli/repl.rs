//! `txtcode repl` — interactive REPL with multiline input and meta-commands.

use crate::builder::{BuildConfig, Builder};
use crate::config::Config;
use crate::lexer::{Lexer, TokenKind};
use crate::parser::Parser;
use crate::runtime::Value;
use crate::validator::Validator;
use std::fs;

pub fn start_repl(
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::{error::ReadlineError, DefaultEditor};

    let mut rl = DefaultEditor::new()?;

    // Load persistent history from ~/.txtcode/repl_history
    let history_path: Option<std::path::PathBuf> = dirs_home()
        .map(|home| home.join(".txtcode").join("repl_history"));
    if let Some(ref p) = history_path {
        let _ = rl.load_history(p); // silently ignore if missing
    }

    if !quiet {
        println!(
            "Txt-code v{}  |  type 'help' for commands, 'exit' to quit",
            env!("CARGO_PKG_VERSION")
        );
    }

    let env_safe_mode = Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false);
    let effective_safe_mode = safe_mode || env_safe_mode;
    let exec_allowed = if allow_exec { true } else { !effective_safe_mode };
    let repl_config = BuildConfig {
        safe_mode: effective_safe_mode,
        allow_exec: exec_allowed,
        debug,
        verbose,
        ..BuildConfig::default()
    };
    let mut vm = Builder::create_repl_vm(&repl_config);

    let mut history: Vec<String> = Vec::new();
    let mut multiline_buf: Vec<String> = Vec::new();
    let mut block_depth: i32 = 0;

    loop {
        let prompt = if block_depth > 0 {
            "...   > "
        } else {
            "txtcode> "
        };
        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                let _ = rl.add_history_entry(trimmed);

                if block_depth == 0 {
                    if trimmed == "exit" || trimmed == "quit" {
                        break;
                    }
                    if trimmed.is_empty() {
                        continue;
                    }

                    if trimmed == "clear" {
                        print!("\x1B[2J\x1B[1;1H");
                        continue;
                    }

                    if let Some(rest) = trimmed.strip_prefix(':') {
                        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                        match parts[0].trim() {
                            "help" | "h" => {
                                let topic = parts.get(1).map(|s| s.trim()).unwrap_or("");
                                repl_help(topic);
                            }
                            "load" => {
                                let path = parts.get(1).map(|s| s.trim()).unwrap_or("");
                                if path.is_empty() {
                                    eprintln!("Usage: :load <file>");
                                } else {
                                    // Validate path before reading (2.5): prevents loading
                                    // files outside the working directory and checks for traversal.
                                    let validated_path = match crate::stdlib::io::IOLib::validate_path_pub(path) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            eprintln!("Path error: {}", e);
                                            continue;
                                        }
                                    };
                                    // File size guard — same 10MB limit as run.rs
                                    match fs::metadata(&validated_path) {
                                        Ok(meta) if meta.len() > 10 * 1024 * 1024 => {
                                            eprintln!("Cannot :load '{}': file too large (max 10 MB)", path);
                                            continue;
                                        }
                                        Err(e) => {
                                            eprintln!("Cannot read '{}': {}", path, e);
                                            continue;
                                        }
                                        _ => {}
                                    }
                                    match Builder::load_and_validate(&validated_path) {
                                        Ok(prog) => {
                                            match vm.interpret(&prog) {
                                                Ok(_) => println!("Loaded: {}", path),
                                                Err(e) => eprintln!("Runtime error: {}", e),
                                            }
                                        }
                                        Err(e) => eprintln!("Error: {}", e),
                                    }
                                }
                            }
                            "save" => {
                                let path = parts.get(1).map(|s| s.trim()).unwrap_or("");
                                if path.is_empty() {
                                    eprintln!("Usage: :save <file>");
                                } else {
                                    match fs::write(path, history.join("\n")) {
                                        Ok(_) => println!("Saved {} line(s) to {}", history.len(), path),
                                        Err(e) => eprintln!("Cannot write '{}': {}", path, e),
                                    }
                                }
                            }
                            "type" => {
                                let expr_src = parts.get(1).map(|s| s.trim()).unwrap_or("");
                                if expr_src.is_empty() {
                                    eprintln!("Usage: :type <expression>");
                                } else {
                                    use crate::typecheck::TypeChecker;
                                    let mut lx = Lexer::new(expr_src.to_string());
                                    match lx.tokenize() {
                                        Ok(toks) => {
                                            let mut p = Parser::new(toks);
                                            match p.parse() {
                                                Ok(prog) => {
                                                    let mut checker = TypeChecker::new();
                                                    match checker.check(&prog) {
                                                        Ok(()) => println!("{} : ok", expr_src),
                                                        Err(errs) => {
                                                            for e in errs {
                                                                println!("type: {}", e);
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => eprintln!("Parse error: {}", e),
                                            }
                                        }
                                        Err(e) => eprintln!("Lex error: {}", e),
                                    }
                                }
                            }
                            "clear" | "reset" => {
                                vm = Builder::create_repl_vm(&repl_config);
                                history.clear();
                                println!("Session cleared.");
                            }
                            other => {
                                eprintln!("Unknown command ':{}'", other);
                                eprintln!("Try :help for a list of commands.");
                            }
                        }
                        continue;
                    }

                    if trimmed == "help" {
                        repl_help("");
                        continue;
                    }
                }

                block_depth += repl_block_delta(trimmed);
                // Guard against underflow: stray 'end' outside a block must not
                // send block_depth negative, which would leave the REPL stuck in
                // a false "inside block" state.
                if block_depth < 0 {
                    block_depth = 0;
                }
                multiline_buf.push(line.clone());

                if block_depth > 0 {
                    continue;
                }

                let source = multiline_buf.join("\n");
                multiline_buf.clear();
                block_depth = 0;

                if source.trim().is_empty() {
                    continue;
                }
                history.push(source.trim().to_string());

                // Hash the current input so integrity checking in interpret_repl
                // can verify the in-memory representation matches what was typed.
                // SecurityLevel reaches Full on Linux/macOS/Windows with this hash.
                vm.runtime_security.hash_and_set_source(source.trim().as_bytes());

                let mut lexer = Lexer::new(source.trim().to_string());
                match lexer.tokenize() {
                    Ok(tokens) => {
                        let tokens: Vec<crate::lexer::Token> = tokens;
                        if tokens.is_empty() {
                            continue;
                        }
                        if tokens.last().is_some_and(|t| t.kind == TokenKind::Eof)
                            && tokens.len() == 1
                        {
                            continue;
                        }
                        let mut parser = Parser::new(tokens);
                        match parser.parse() {
                            Ok(program) => {
                                if let Err(e) = Validator::validate_program(&program) {
                                    eprintln!("Validation error: {}", e);
                                    continue;
                                }
                                match vm.interpret_repl(&program) {
                                    Ok(value) => {
                                        if !matches!(value, Value::Null) {
                                            println!("{}", Value::to_string(&value));
                                            vm.define_global("_".to_string(), value);
                                        }
                                    }
                                    Err(e) => eprintln!("Runtime error: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Parse error: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Lex error: {}", e),
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                if block_depth > 0 {
                    multiline_buf.clear();
                    block_depth = 0;
                    println!("\n(multiline input cancelled)");
                } else {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }

    // Save persistent history on exit (up to 1000 entries)
    if let Some(ref p) = history_path {
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = rl.save_history(p);
    }

    Ok(())
}

/// Return the user's home directory path, or None.
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(std::path::PathBuf::from))
}

fn repl_help(topic: &str) {
    match topic {
        "" | "help" => {
            println!("Txt-code REPL — available commands:");
            println!();
            println!("  exit / quit          Exit the REPL (also Ctrl+D)");
            println!("  clear                Clear the screen");
            println!("  help                 Show this help");
            println!();
            println!("  :load <file>         Execute a .tc file into this session");
            println!("  :save <file>         Save session history to a file");
            println!("  :type <expr>         Show inferred type of an expression");
            println!("  :clear               Reset all variables (fresh session)");
            println!("  :help [topic]        Show help (topics: syntax, stdlib, types, ops)");
            println!();
            println!("  _                    Last evaluated result");
            println!();
            println!("  Multiline: open a block (if/for/define/try/match), close with 'end'");
            println!("  Prompt changes to '...   >' while inside a block.");
        }
        "syntax" => {
            println!("── Syntax ───────────────────────────────────────────────");
            println!("  store → x → 42          assign variable");
            println!("  store → arr[0] → 99      index assign");
            println!("  x += 5                   compound assign (+=  -=  *=  /=  **=)");
            println!("  if → cond                if block");
            println!("  elseif → cond            else-if (one keyword)");
            println!("  else                     else");
            println!("  end                      close any block");
            println!("  for → x in arr           for loop");
            println!("  while → cond             while loop");
            println!("  define → f(a, b)         function definition");
            println!("  return → value           return");
            println!("  cond ? a : b             ternary");
            println!("  x |> func                pipe: func(x)");
            println!("  try / catch e / end      error handling");
            println!("  match → x                pattern match");
            println!("  import → module          import module");
            println!("  struct Point(x, y)       struct definition");
        }
        "types" => {
            println!("── Types ─────────────────────────────────────────────────");
            println!("  42          Integer");
            println!("  3.14        Float");
            println!("  \"hello\"     String");
            println!("  true/false  Boolean");
            println!("  null        Null");
            println!("  [1, 2, 3]   Array");
            println!("  {{a: 1}}     Map");
            println!("  ok(v)       Result::Ok");
            println!("  err(e)      Result::Err");
            println!("  r\"\\n\"      Raw string (no escape processing)");
            println!("  \"\"\"...\"\"\"  Multiline string");
            println!("  1_000_000   Number with separators");
        }
        "ops" => {
            println!("── Operators ─────────────────────────────────────────────");
            println!("  +  -  *  /  %  **       arithmetic");
            println!("  ==  !=  <  >  <=  >=    comparison");
            println!("  and  or  not             logical");
            println!("  &  |  ^  ~  <<  >>       bitwise");
            println!("  ??                       null coalesce");
            println!("  ?.  ?[]                  optional chaining");
            println!("  |>                       pipe");
            println!("  ++x  --x                 prefix increment/decrement");
        }
        "stdlib" => {
            println!("── Standard Library ──────────────────────────────────────");
            println!("  String:  len, upper, lower, trim, split, replace, contains,");
            println!("           starts_with, ends_with, substr, to_int, to_float");
            println!("  Array:   push, pop, len, map, filter, reduce, sort, reverse,");
            println!("           join, contains, slice, flat_map, zip, enumerate");
            println!("  Math:    abs, sqrt, pow, floor, ceil, round, min, max,");
            println!("           sin, cos, tan, log, exp, pi, random");
            println!("           math_clamp, math_gcd, math_lcm, math_factorial");
            println!("  IO:      print, println, read_file, write_file, read_lines,");
            println!("           read_csv, temp_file, watch_file");
            println!("  Net:     http_get, http_post, http_put, http_delete,");
            println!("           http_headers, http_status, http_timeout");
            println!("  Sys:     env_get, env_set, env_list, exec, pipe_exec,");
            println!("           which, is_root, cpu_count, os_name, os_version");
            println!("  Crypto:  sha256, md5, hmac_sha256, uuid_v4, base64_encode,");
            println!("           base64_decode, pbkdf2, ed25519_sign, ed25519_verify");
            println!("  Result:  ok(v), err(e), is_ok(r), is_err(r), unwrap(r),");
            println!("           unwrap_or(r, default)");
            println!("  JSON:    json_encode, json_decode");
        }
        other => {
            eprintln!(
                "No help for '{}'. Topics: syntax, types, ops, stdlib",
                other
            );
        }
    }
}

/// Count net block-depth delta for one line (used for REPL multiline detection).
fn repl_block_delta(line: &str) -> i32 {
    let t = line.trim();
    if t.starts_with('#') {
        return 0;
    }
    let first = t
        .split(|c: char| c.is_whitespace() || c == '→')
        .next()
        .unwrap_or("");
    match first {
        "if" | "while" | "for" | "foreach" | "define" | "def" | "async" | "try" | "match"
        | "switch" | "do" | "repeat" | "struct" | "enum" => 1,
        "end" => -1,
        _ => 0,
    }
}
