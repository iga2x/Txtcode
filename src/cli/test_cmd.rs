//! `txtcode test` — run test files, watch mode.

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime::vm::VirtualMachine;
use crate::validator::Validator;
use std::fs;
use std::path::PathBuf;

pub fn run_tests(
    path: &PathBuf,
    filter: Option<&str>,
    json_out: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut test_files: Vec<PathBuf> = Vec::new();

    if path.is_file() {
        test_files.push(path.clone());
    } else if path.is_dir() {
        collect_test_files(path, &mut test_files)?;
    } else {
        return Err(format!("Test path '{}' not found", path.display()).into());
    }

    if let Some(f) = filter {
        test_files.retain(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(f))
        });
    }

    if test_files.is_empty() {
        if json_out {
            println!("{{\"passed\":0,\"failed\":0,\"tests\":[]}}");
        } else {
            println!(
                "No test files found in '{}'. Test files must be named test_*.tc or *_test.tc",
                path.display()
            );
        }
        return Ok(());
    }

    if !json_out {
        println!("Running {} test file(s)...\n", test_files.len());
    }

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut json_tests: Vec<String> = Vec::new();

    for test_file in &test_files {
        let name = test_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let source = fs::read_to_string(test_file)?;
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                let msg = format!("lex error: {}", e);
                if json_out {
                    json_tests.push(format!(
                        "{{\"name\":\"{}\",\"passed\":false,\"error\":\"{}\"}}",
                        name,
                        msg.replace('"', "\\\"")
                    ));
                } else {
                    println!("  FAIL  {} — {}", name, msg);
                }
                failed += 1;
                continue;
            }
        };
        let mut parser = Parser::new(tokens);
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("parse error: {}", e);
                if json_out {
                    json_tests.push(format!(
                        "{{\"name\":\"{}\",\"passed\":false,\"error\":\"{}\"}}",
                        name,
                        msg.replace('"', "\\\"")
                    ));
                } else {
                    println!("  FAIL  {} — {}", name, msg);
                }
                failed += 1;
                continue;
            }
        };
        if let Err(e) = Validator::validate_program(&program) {
            let msg = format!("validation error: {}", e);
            if json_out {
                json_tests.push(format!(
                    "{{\"name\":\"{}\",\"passed\":false,\"error\":\"{}\"}}",
                    name,
                    msg.replace('"', "\\\"")
                ));
            } else {
                println!("  FAIL  {} — {}", name, msg);
            }
            failed += 1;
            continue;
        }
        let mut vm = VirtualMachine::new();
        match vm.interpret(&program) {
            Ok(_) => {
                if json_out {
                    json_tests.push(format!(
                        "{{\"name\":\"{}\",\"passed\":true,\"error\":null}}",
                        name
                    ));
                } else {
                    println!("  PASS  {}", name);
                }
                passed += 1;
            }
            Err(e) => {
                let msg = e.to_string();
                if json_out {
                    json_tests.push(format!(
                        "{{\"name\":\"{}\",\"passed\":false,\"error\":\"{}\"}}",
                        name,
                        msg.replace('"', "\\\"")
                    ));
                } else {
                    println!("  FAIL  {} — {}", name, msg);
                }
                failed += 1;
            }
        }
    }

    if json_out {
        println!(
            "{{\"passed\":{},\"failed\":{},\"tests\":[{}]}}",
            passed,
            failed,
            json_tests.join(",")
        );
    } else {
        println!("\n{} passed, {} failed", passed, failed);
    }

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

pub fn run_tests_watch(
    path: &PathBuf,
    filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Watching for changes in '{}' (Ctrl+C to stop)...\n",
        path.display()
    );

    let snapshot =
        |path: &PathBuf| -> std::collections::HashMap<PathBuf, std::time::SystemTime> {
            let mut map = std::collections::HashMap::new();
            let mut queue = vec![path.clone()];
            while let Some(dir) = queue.pop() {
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            queue.push(p);
                        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                            if ext == "tc" {
                                if let Ok(meta) = fs::metadata(&p) {
                                    if let Ok(modified) = meta.modified() {
                                        map.insert(p, modified);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            map
        };

    let mut prev = snapshot(path);
    let _ = run_tests(path, filter, false);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let current = snapshot(path);
        let changed = current.iter().any(|(p, t)| prev.get(p) != Some(t))
            || prev.keys().any(|p| !current.contains_key(p));
        if changed {
            println!("\n── file change detected, re-running tests ──\n");
            let _ = run_tests(path, filter, false);
            prev = current;
        }
    }
}

fn collect_test_files(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files(&path, files)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if (name.starts_with("test_") || name.ends_with("_test.tc")) && name.ends_with(".tc") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(())
}
