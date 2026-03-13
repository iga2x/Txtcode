//! `txtcode format` — format Txt-code source files.

use std::fs;
use std::path::PathBuf;

pub fn format_files(
    files: &[PathBuf],
    write: bool,
    check: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut needs_format: Vec<PathBuf> = Vec::new();
    let mut json_results: Vec<String> = Vec::new();

    for file in files {
        let source = fs::read_to_string(file)?;
        let formatted = crate::tools::formatter::Formatter::format_source(&source)?;
        let changed = source != formatted;

        if json {
            json_results.push(format!(
                "{{\"file\":\"{}\",\"changed\":{}}}",
                file.display(),
                changed
            ));
        } else if check {
            if changed {
                println!("needs formatting: {}", file.display());
                needs_format.push(file.clone());
            }
        } else if write {
            if changed {
                fs::write(file, &formatted)?;
                println!("  formatted  {}", file.display());
            }
        } else {
            print!("{}", formatted);
        }
    }

    if json {
        println!("[{}]", json_results.join(",\n"));
        if check && json_results.iter().any(|r| r.contains("\"changed\":true")) {
            std::process::exit(1);
        }
        return Ok(());
    }

    if check && !needs_format.is_empty() {
        eprintln!(
            "\n{} file(s) need formatting. Run with --write to fix.",
            needs_format.len()
        );
        std::process::exit(1);
    }
    Ok(())
}
