//! `txtcode compile` / `txtcode inspect` — compile to bytecode and inspect bytecode files.

#[cfg(feature = "bytecode")]
use crate::compiler::bytecode::BytecodeCompiler;
#[cfg(feature = "bytecode")]
use crate::compiler::optimizer::{OptimizationLevel, Optimizer};
#[cfg(feature = "bytecode")]
use crate::config::Config;
#[cfg(feature = "bytecode")]
use crate::lexer::Lexer;
#[cfg(feature = "bytecode")]
use crate::parser::Parser;
#[cfg(feature = "bytecode")]
use crate::tools::logger;
#[cfg(feature = "bytecode")]
use crate::validator::Validator;
#[cfg(feature = "bytecode")]
use sha2::{Digest, Sha256};
#[cfg(feature = "bytecode")]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(not(feature = "bytecode"))]
pub fn compile_file(
    _file: &PathBuf,
    _output: Option<&PathBuf>,
    _optimize: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("The 'compile' command requires the 'bytecode' feature. \
         Rebuild with: cargo build --features bytecode"
        .into())
}

#[cfg(not(feature = "bytecode"))]
pub fn inspect_bytecode(
    _file: &Path,
    _format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("The 'inspect' command requires the 'bytecode' feature. \
         Rebuild with: cargo build --features bytecode"
        .into())
}

#[cfg(feature = "bytecode")]
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
        "basic" => OptimizationLevel::Basic,
        "aggressive" => {
            // Aggressive optimization is planned for v0.7.0. Returning an error
            // rather than silently falling back ensures the exit code is non-zero
            // when the requested level was not applied (6.4 fix).
            return Err(
                "Optimization level 'aggressive' is not yet implemented. \
                 Use 'none' or 'basic'. (Planned for v0.7.0)"
                    .into(),
            );
        }
        other => {
            return Err(format!(
                "Unknown optimization level '{}'. Valid options: none, basic",
                other
            )
            .into());
        }
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

#[cfg(feature = "bytecode")]
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

#[cfg(feature = "bytecode")]
fn generate_cache_key(source: &str, optimize: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(optimize.as_bytes());
    let hash = hasher.finalize();
    Ok(hex::encode(&hash[..16]))
}
