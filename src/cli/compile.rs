//! `txtcode compile` / `txtcode inspect` — compile to bytecode and inspect bytecode files.

#[cfg(feature = "bytecode")]
use crate::builder::{BuildConfig, BuildTarget, Builder};
#[cfg(feature = "bytecode")]
use crate::config::Config;
#[cfg(feature = "bytecode")]
use crate::tools::logger;
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

    // Cache hit: copy artifact without re-compiling.
    if user_config.package.cache_packages {
        let source = fs::read_to_string(file)?;
        let cache_key = generate_cache_key(&source, optimize)?;
        let cache_path = Config::get_cache_path(&cache_key)?;
        if cache_path.exists() {
            logger::log_info(&format!("Using cached bytecode for: {}", file.display()));
            let output_path = output.cloned().unwrap_or_else(|| file.with_extension("txtc"));
            fs::copy(&cache_path, &output_path)?;
            println!("Compiled (from cache) to: {}", output_path.display());
            return Ok(());
        }
    }

    logger::log_info(&format!("Compiling: {}", file.display()));

    match optimize {
        "none" | "basic" => {}
        "aggressive" => {
            return Err("Optimization level 'aggressive' is not yet implemented. \
                 Use 'none' or 'basic'. (Planned for v0.7.0)".into());
        }
        other => {
            return Err(format!(
                "Unknown optimization level '{}'. Valid options: none, basic", other
            ).into());
        }
    }

    let config = BuildConfig {
        input: file.clone(),
        output: output.map(|p| p.clone()),
        target: BuildTarget::Bytecode,
        optimize: optimize == "basic",
        type_check: true,
        ..Default::default()
    };

    let build_output = Builder::build(&config)?;

    // Write to cache if enabled.
    if user_config.package.cache_packages {
        if let Some(artifact_path) = &build_output.artifact_path {
            let source = fs::read_to_string(file)?;
            let cache_key = generate_cache_key(&source, optimize)?;
            let cache_path = Config::get_cache_path(&cache_key)?;
            if let Some(parent) = cache_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = fs::copy(artifact_path, &cache_path);
        }
    }

    if let Some(artifact_path) = &build_output.artifact_path {
        println!("Compiled to: {}", artifact_path.display());
    }
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

#[cfg(not(feature = "bytecode"))]
pub fn compile_wasm(
    _file: &PathBuf,
    _output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("The 'compile --target wasm' command requires the 'bytecode' feature. \
         Rebuild with: cargo build --features bytecode"
        .into())
}

/// Task 12.3 / 29.2 — Compile a Txt-code file to WebAssembly.
///
/// `binary`: when true produce a `.wasm` binary (requires `wasm` feature);
///           when false produce a `.wat` text file.
#[cfg(feature = "bytecode")]
pub fn compile_wasm(
    file: &PathBuf,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    compile_wasm_with_opts(file, output, false)
}

/// Compile to WAT or binary WASM depending on `binary`.
#[cfg(feature = "bytecode")]
pub fn compile_wasm_with_opts(
    file: &PathBuf,
    output: Option<&std::path::Path>,
    binary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let target = if binary {
        #[cfg(feature = "wasm")]
        { BuildTarget::WasmBinary }
        #[cfg(not(feature = "wasm"))]
        return Err("Binary WASM output requires the 'wasm' feature. Rebuild with: cargo build --features wasm".into())
    } else {
        BuildTarget::WasmText
    };

    let config = BuildConfig {
        input: file.clone(),
        output: output.map(|p| p.to_path_buf()),
        target,
        type_check: true,
        ..Default::default()
    };

    let build_output = Builder::build(&config)?;

    if let Some(artifact_path) = &build_output.artifact_path {
        if binary {
            println!("Compiled (WASM binary) to: {}", artifact_path.display());
            println!("  {} bytes written", std::fs::metadata(artifact_path).map(|m| m.len()).unwrap_or(0));
        } else {
            println!("Compiled (WASM/WAT) to: {}", artifact_path.display());
            println!("  To convert to binary: wat2wasm {} -o {}",
                artifact_path.display(),
                artifact_path.with_extension("wasm").display());
        }
    }
    Ok(())
}
