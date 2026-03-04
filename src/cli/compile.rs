use std::fs;
use std::path::Path;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::compiler::bytecode::BytecodeCompiler;
use crate::compiler::optimizer::{Optimizer, OptimizationLevel};
use crate::security::{Obfuscator, BytecodeEncryptor};
use crate::typecheck::checker::TypeChecker;
use serde_json;

/// Compile a Txtcode file
pub fn compile_file(
    input: &Path,
    output: Option<&Path>,
    optimize: &str,
    obfuscate: bool,
    encrypt: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read source file
    let source = fs::read_to_string(input)?;
    
    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    // Type check (optional, but recommended)
    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(&program) {
        eprintln!("Type checking errors:");
        for error in errors {
            eprintln!("  - {}", error);
        }
        // Continue anyway for now
    }
    
    // Compile to bytecode
    let mut compiler = BytecodeCompiler::new();
    let mut bytecode = compiler.compile(&program);
    
    // Optimize
    let opt_level = match optimize {
        "none" => OptimizationLevel::None,
        "basic" => OptimizationLevel::Basic,
        "aggressive" => {
            // Aggressive optimization removed - fall back to basic
            eprintln!("Warning: 'aggressive' optimization level removed. Using 'basic' instead.");
            OptimizationLevel::Basic
        },
        _ => OptimizationLevel::Basic,
    };
    
    let optimizer = Optimizer::new(opt_level);
    bytecode = optimizer.optimize_bytecode(&bytecode)?;
    
    // Obfuscate (if requested)
    if obfuscate {
        let _obfuscator = Obfuscator::new();
        // Obfuscation would modify the bytecode
        // For now, this is a placeholder
    }
    
    // Determine output path
    let output_path = if let Some(out) = output {
        out.to_path_buf()
    } else {
        let mut out = input.to_path_buf();
        out.set_extension("tcb"); // Txtcode Bytecode
        out
    };
    
    // Serialize bytecode
    let serialized_json = serde_json::to_string_pretty(&bytecode)?;
    let serialized_bytes = bincode::serialize(&bytecode)?;
    
    // Encrypt (if requested)
    if encrypt {
        let encryptor = BytecodeEncryptor::new();
        let encrypted = encryptor.encrypt(&serialized_bytes)?;
        fs::write(&output_path, encrypted.serialize())?;
        println!("Compiled and encrypted to: {}", output_path.display());
    } else {
        // Save as JSON for readability, or binary for efficiency
        fs::write(&output_path, serialized_json)?;
        println!("Compiled to: {}", output_path.display());
    }
    
    Ok(())
}
