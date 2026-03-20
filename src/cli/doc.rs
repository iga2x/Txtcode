//! `txtcode doc` — generate documentation from source files.

use std::fs;
use std::path::PathBuf;

pub fn generate_docs(
    files: &[PathBuf],
    output: Option<&PathBuf>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::tools::docgen::{DocGenerator, OutputFormat};

    let fmt = match format {
        "html" => OutputFormat::Html,
        "json" => OutputFormat::Json,
        _ => OutputFormat::Markdown,
    };
    let gen = DocGenerator::with_format(fmt);
    let ext = match format { "html" => "html", "json" => "json", _ => "md" };

    let default_out = PathBuf::from("docs/api");
    let out_dir = output.unwrap_or(&default_out);
    fs::create_dir_all(out_dir)?;

    if files.is_empty() {
        let src = PathBuf::from("src");
        if src.is_dir() {
            let mut discovered = Vec::new();
            collect_tc_files(&src, &mut discovered)?;
            if discovered.is_empty() {
                println!("No .tc files found in src/");
                return Ok(());
            }
            return generate_docs(&discovered, output, format);
        }
        println!("No input files specified and no src/ directory found.");
        return Ok(());
    }

    for file in files {
        if !file.exists() {
            eprintln!("Warning: '{}' not found, skipping", file.display());
            continue;
        }
        let source = fs::read_to_string(file)?;
        let doc = gen.generate_docs_from_source(&source);

        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
        let out_path = out_dir.join(format!("{}.{}", stem, ext));
        fs::write(&out_path, &doc)?;
        println!("  generated  {}", out_path.display());
    }

    println!("\nDocumentation written to {}/", out_dir.display());
    Ok(())
}

fn collect_tc_files(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_tc_files(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "tc" {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(())
}
