//! `txtcode check` — lint + type-check without executing.

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::validator::Validator;
use std::fs;
use std::path::PathBuf;

pub fn check_files(files: &[PathBuf], json_out: bool) -> Result<(), Box<dyn std::error::Error>> {
    use crate::tools::linter::{Linter, Severity};
    use crate::typecheck::inference::TypeInference;

    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut json_issues: Vec<String> = Vec::new();

    for file in files {
        if !file.exists() {
            let msg = format!("File '{}' not found", file.display());
            if json_out {
                json_issues.push(format!(
                    "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                    file.display(),
                    msg
                ));
            } else {
                eprintln!("{}", msg);
            }
            total_errors += 1;
            continue;
        }

        let source = fs::read_to_string(file)?;

        let issues =
            Linter::lint_source_with_path(&source, Some(file.as_path())).unwrap_or_default();

        for issue in &issues {
            match issue.severity {
                Severity::Error => total_errors += 1,
                Severity::Warning => total_warnings += 1,
                Severity::Info => {}
            }
            if json_out {
                json_issues.push(format!(
                    "{{\"file\":\"{}\",\"line\":{},\"col\":{},\"severity\":\"{}\",\"message\":\"{}\"}}",
                    file.display(),
                    issue.line,
                    issue.column,
                    issue.severity,
                    issue.message.replace('"', "\\\"")
                ));
            } else {
                let prefix = match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                };
                println!(
                    "  [{}] {}:{}:{} — {}",
                    prefix,
                    file.display(),
                    issue.line,
                    issue.column,
                    issue.message
                );
            }
        }

        let mut lexer = Lexer::new(source);
        if let Ok(tokens) = lexer.tokenize() {
            let mut parser = Parser::new(tokens);
            // Use parse_with_errors so all syntax errors are reported at once.
            let (program, parse_errors) = parser.parse_with_errors();
            for parse_err in &parse_errors {
                total_errors += 1;
                if json_out {
                    json_issues.push(format!(
                        "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                        file.display(),
                        parse_err.replace('"', "\\\"")
                    ));
                } else {
                    println!("  [syntax] {}", parse_err);
                }
            }
            if parse_errors.is_empty() {
                // Run the validator pipeline (syntax → semantics → restrictions).
                if let Err(validation_err) = Validator::validate_program(&program) {
                    total_errors += 1;
                    let msg = validation_err.to_string();
                    if json_out {
                        json_issues.push(format!(
                            "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                            file.display(),
                            msg.replace('"', "\\\"")
                        ));
                    } else {
                        println!("  [validation-error] {} — {}", file.display(), msg);
                    }
                }

                let mut infer = TypeInference::new();
                if let Err(type_errs) = infer.infer_program(&program) {
                    for msg in &type_errs {
                        total_errors += 1;
                        if json_out {
                            json_issues.push(format!(
                                "{{\"file\":\"{}\",\"line\":0,\"col\":0,\"severity\":\"error\",\"message\":\"{}\"}}",
                                file.display(),
                                msg.replace('"', "\\\"")
                            ));
                        } else {
                            println!("  [type-error] {} — {}", file.display(), msg);
                        }
                    }
                }
            }
        }
    }

    if json_out {
        println!("[{}]", json_issues.join(",\n"));
    } else if total_errors == 0 && total_warnings == 0 {
        println!("No issues found in {} file(s).", files.len());
    } else {
        println!(
            "\n{} error(s), {} warning(s) across {} file(s).",
            total_errors,
            total_warnings,
            files.len()
        );
    }

    if total_errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
