//! `txtcode lint` — lint Txt-code source files.

use std::fs;
use std::path::PathBuf;

pub fn lint_files(
    files: &[PathBuf],
    format: &str,
    fix: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::tools::linter::{Linter, Severity};
    let mut error_count = 0usize;
    let mut warning_count = 0usize;
    let json_out = format == "json";
    let mut json_issues: Vec<String> = Vec::new();

    for file in files {
        let source = fs::read_to_string(file)?;
        let issues = Linter::lint_source_with_path(&source, Some(file.as_path()))?;

        if fix && !issues.is_empty() {
            let fixed = lint_autofix(&source);
            if fixed != source {
                fs::write(file, &fixed)?;
                if !json_out {
                    println!("  fixed  {}", file.display());
                }
            }
        }

        for issue in &issues {
            match issue.severity {
                Severity::Error => error_count += 1,
                Severity::Warning => warning_count += 1,
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
    }

    if json_out {
        println!("[{}]", json_issues.join(",\n"));
        return Ok(());
    }

    if error_count > 0 || warning_count > 0 {
        println!("\n{} error(s), {} warning(s)", error_count, warning_count);
        if error_count > 0 {
            std::process::exit(1);
        }
    } else {
        println!("No issues found");
    }

    Ok(())
}

/// Auto-fix simple lint issues: trailing whitespace and 3+ consecutive blank lines.
fn lint_autofix(source: &str) -> String {
    let mut result = String::new();
    let mut blank_run = 0usize;
    for line in source.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                result.push('\n');
            }
        } else {
            blank_run = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    result
}
