//! `txtcode check` — lint + type-check without executing.
//!
//! Pipeline stages (lex → parse → validate → type-check) are delegated to
//! [`crate::builder::Builder::check`].  This file owns CLI concerns only:
//!   - Linter integration (linter is a tool, not a pipeline stage)
//!   - JSON output formatting
//!   - Exit code management

use crate::builder::{BuildConfig, Builder};
use std::fs;
use std::path::PathBuf;

pub fn check_files(files: &[PathBuf], json_out: bool) -> Result<(), Box<dyn std::error::Error>> {
    use crate::tools::linter::{Linter, Severity};

    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut json_issues: Vec<String> = Vec::new();

    for file in files {
        if !file.exists() {
            let msg = format!("File '{}' not found", file.display());
            if json_out {
                json_issues.push(format!(
                    "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                    file.display(), msg
                ));
            } else {
                eprintln!("{}", msg);
            }
            total_errors += 1;
            continue;
        }

        let source = fs::read_to_string(file)?;

        // ── Linter (tool concern, kept in CLI) ────────────────────────────────
        let issues = Linter::lint_source_with_path(&source, Some(file.as_path())).unwrap_or_default();
        for issue in &issues {
            match issue.severity {
                Severity::Error => total_errors += 1,
                Severity::Warning => total_warnings += 1,
                Severity::Info => {}
            }
            if json_out {
                json_issues.push(format!(
                    "{{\"file\":\"{}\",\"line\":{},\"col\":{},\"severity\":\"{}\",\"message\":\"{}\"}}",
                    file.display(), issue.line, issue.column, issue.severity,
                    issue.message.replace('"', "\\\"")
                ));
            } else {
                let prefix = match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                };
                println!("  [{}] {}:{}:{} — {}", prefix, file.display(), issue.line, issue.column, issue.message);
            }
        }

        // ── Builder pipeline (lex → parse → validate → type-check) ───────────
        let config = BuildConfig {
            input: file.clone(),
            type_check: true,
            strict_types: false, // check always advisory; callers use --strict-types on run
            ..Default::default()
        };

        match Builder::check(&config) {
            Ok(report) => {
                for msg in &report.errors {
                    total_errors += 1;
                    if json_out {
                        json_issues.push(format!(
                            "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                            file.display(), msg.replace('"', "\\\"")
                        ));
                    } else {
                        println!("  [error] {} — {}", file.display(), msg);
                    }
                }
                for msg in &report.warnings {
                    total_warnings += 1;
                    if json_out {
                        json_issues.push(format!(
                            "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"warning\",\"message\":\"{}\"}}",
                            file.display(), msg.replace('"', "\\\"")
                        ));
                    } else {
                        println!("  [warning] {} — {}", file.display(), msg);
                    }
                }
            }
            Err(e) => {
                total_errors += 1;
                if json_out {
                    json_issues.push(format!(
                        "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                        file.display(), e.replace('"', "\\\"")
                    ));
                } else {
                    println!("  [error] {} — {}", file.display(), e);
                }
            }
        }
    }

    if json_out {
        println!("[{}]", json_issues.join(",\n"));
    } else if total_errors == 0 && total_warnings == 0 {
        println!("No issues found in {} file(s).", files.len());
    } else {
        println!("\n{} error(s), {} warning(s) across {} file(s).", total_errors, total_warnings, files.len());
    }

    if total_errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
