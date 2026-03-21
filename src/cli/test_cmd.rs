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
        let test_result = vm.interpret(&program).and_then(|val| {
            // If the program returned a Future (e.g. async test without top-level await),
            // resolve it now so async test files work transparently.
            match val {
                crate::runtime::core::Value::Future(handle) => {
                    handle.resolve().map_err(crate::runtime::errors::RuntimeError::new)
                }
                other => Ok(other),
            }
        });
        match test_result {
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

pub fn run_tests_with_coverage(
    path: &PathBuf,
    filter: Option<&str>,
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
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains(f))
        });
    }

    // Per-file: (source_lines, covered_lines_set)
    struct FileCoverage {
        name: String,
        total_lines: usize,
        covered: std::collections::HashSet<u32>,
        passed: bool,
    }

    let mut file_coverages: Vec<FileCoverage> = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;

    for test_file in &test_files {
        let name = test_file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        let source = fs::read_to_string(test_file)?;
        let total_lines = source.lines().count();
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                println!("  FAIL  {} — lex error: {}", name, e);
                failed += 1;
                file_coverages.push(FileCoverage {
                    name, total_lines, covered: Default::default(), passed: false,
                });
                continue;
            }
        };
        let mut parser = crate::parser::Parser::new(tokens);
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => {
                println!("  FAIL  {} — parse error: {}", name, e);
                failed += 1;
                file_coverages.push(FileCoverage {
                    name, total_lines, covered: Default::default(), passed: false,
                });
                continue;
            }
        };

        let mut vm = crate::runtime::vm::VirtualMachine::new();
        vm.enable_coverage();
        let ok = vm.interpret(&program).is_ok();
        let covered = vm.covered_lines.clone();

        if ok { passed += 1; } else { failed += 1; }
        println!("  {}  {} ({}/{} lines)", if ok { "PASS" } else { "FAIL" }, name, covered.len(), total_lines);
        file_coverages.push(FileCoverage { name, total_lines, covered, passed: ok });
    }

    // Summary to stdout
    let total_lines: usize = file_coverages.iter().map(|f| f.total_lines).sum();
    let total_covered: usize = file_coverages.iter().map(|f| f.covered.len()).sum();
    let pct = if total_lines > 0 { 100 * total_covered / total_lines } else { 0 };
    println!("\n{} passed, {} failed", passed, failed);
    println!("Coverage: {}/{} lines ({}%)", total_covered, total_lines, pct);

    // Generate coverage/index.html
    fs::create_dir_all("coverage")?;
    let mut html = String::from("<!DOCTYPE html><html><head><meta charset='utf-8'>");
    html.push_str("<title>Txtcode Coverage Report</title>");
    html.push_str("<style>body{font-family:monospace;padding:1em}");
    html.push_str(".cov{background:#cfc}.miss{background:#fcc}.hdr{background:#eee;padding:.3em .6em}");
    html.push_str("table{border-collapse:collapse;width:100%}td,th{padding:.2em .5em;border:1px solid #ccc}</style></head><body>");
    html.push_str(&format!("<h1>Coverage Report</h1><p>{} passed, {} failed | Coverage: {}/{}  lines ({}%)</p>", passed, failed, total_covered, total_lines, pct));

    for fc in &file_coverages {
        html.push_str(&format!("<h2>{} — {}/{} lines ({}%)</h2>", fc.name, fc.covered.len(), fc.total_lines,
            if fc.total_lines > 0 { 100 * fc.covered.len() / fc.total_lines } else { 0 }));
        html.push_str("<table><tr><th>Line</th><th>Source</th></tr>");
        let src = test_files.iter()
            .find(|p| p.file_stem().and_then(|s| s.to_str()) == Some(&fc.name))
            .and_then(|p| fs::read_to_string(p).ok())
            .unwrap_or_default();
        for (i, line) in src.lines().enumerate() {
            let lineno = (i + 1) as u32;
            let cls = if fc.covered.contains(&lineno) { "cov" } else { "miss" };
            let escaped = line.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            html.push_str(&format!("<tr class='{}'><td>{}</td><td><pre>{}</pre></td></tr>", cls, lineno, escaped));
        }
        html.push_str("</table>");
    }
    html.push_str("</body></html>");
    fs::write("coverage/index.html", &html)?;
    println!("Coverage report: coverage/index.html");

    if failed > 0 { std::process::exit(1); }
    Ok(())
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
