//! `txtcode compile` / `txtcode inspect` — compile to bytecode and inspect bytecode files.

use crate::compiler::bytecode::BytecodeCompiler;
use crate::compiler::optimizer::{OptimizationLevel, Optimizer};
use crate::config::Config;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::tools::logger;
use crate::validator::Validator;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub fn compile_file(
    file: &PathBuf,
    output: Option<&PathBuf>,
    optimize: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let user_config = Config::load_config().unwrap_or_default();
    let optimize = if optimize == "basic" && user_config.compiler.optimization != "basic" {
        user_config.compiler.optimization.as_str()
    } else {
        optimize
    };

    let source = fs::read_to_string(file)?;

    if user_config.package.cache_packages {
        let cache_key = generate_cache_key(&source, optimize)?;
        let cache_path = Config::get_cache_path(&cache_key)?;
        if cache_path.exists() {
            logger::log_info(&format!("Using cached bytecode for: {}", file.display()));
            let output_path = output
                .cloned()
                .unwrap_or_else(|| file.with_extension("txtc"));
            fs::copy(&cache_path, &output_path)?;
            println!("Compiled (from cache) to: {}", output_path.display());
            return Ok(());
        }
    }

    logger::log_info(&format!("Compiling: {}", file.display()));

    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;

    // Validate before compiling to bytecode — catches security issues and
    // semantic errors that would only surface at runtime otherwise.
    Validator::validate_program(&program)?;

    let opt_level = match optimize {
        "none" => OptimizationLevel::None,
        "aggressive" => {
            eprintln!("Warning: 'aggressive' optimization not implemented. Using 'basic'.");
            OptimizationLevel::Basic
        }
        _ => OptimizationLevel::Basic,
    };
    let optimizer = Optimizer::new(opt_level);
    optimizer.optimize_ast(&mut program);

    let mut compiler = BytecodeCompiler::new();
    let bytecode_program = compiler.compile(&program);
    let serialized = bincode::serialize(&bytecode_program)?;

    if user_config.package.cache_packages {
        let cache_key = generate_cache_key(&source, optimize)?;
        let cache_path = Config::get_cache_path(&cache_key)?;
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cache_path, &serialized)?;
    }

    let output_path = output
        .cloned()
        .unwrap_or_else(|| file.with_extension("txtc"));
    fs::write(&output_path, serialized)?;
    println!("Compiled to: {}", output_path.display());
    Ok(())
}

pub fn inspect_bytecode(file: &Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::compiler::bytecode::Bytecode;
    let bytes = std::fs::read(file)?;
    let bytecode: Bytecode = bincode::deserialize(&bytes).map_err(|e| {
        format!(
            "Failed to deserialize bytecode: {}. Is this a compiled .txtc file?",
            e
        )
    })?;
    match format {
        "json" => {
            println!("[");
            let last = bytecode.instructions.len().saturating_sub(1);
            for (i, instr) in bytecode.instructions.iter().enumerate() {
                let comma = if i < last { "," } else { "" };
                println!("  {{\"addr\":{},\"op\":{:?}}}{}", i, instr, comma);
            }
            println!("]");
        }
        _ => {
            println!("=== Bytecode: {} ===", file.display());
            println!("Instructions: {}", bytecode.instructions.len());
            println!("Constants: {}", bytecode.constants.len());
            println!("---");
            for (i, instr) in bytecode.instructions.iter().enumerate() {
                println!("{:04}  {:?}", i, instr);
            }
        }
    }
    Ok(())
}

fn generate_cache_key(source: &str, optimize: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(optimize.as_bytes());
    let hash = hasher.finalize();
    Ok(hex::encode(&hash[..16]))
}
