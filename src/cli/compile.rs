use crate::compiler::bytecode::BytecodeCompiler;
use crate::compiler::optimizer::{OptimizationLevel, Optimizer};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::security::BytecodeEncryptor;
use crate::typecheck::checker::TypeChecker;
use std::fs;
use std::path::Path;

/// Compile a Txtcode file
pub fn compile_file(
    input: &Path,
    output: Option<&Path>,
    optimize: &str,
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

    // Type check
    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(&program) {
        eprintln!("Type checking errors:");
        for error in &errors {
            eprintln!("  - {}", error);
        }
        return Err(format!("{} type error(s) — compilation aborted", errors.len()).into());
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
        }
        _ => OptimizationLevel::Basic,
    };

    let optimizer = Optimizer::new(opt_level);
    bytecode = optimizer.optimize_bytecode(&bytecode)?;

    // Determine output path
    let output_path = if let Some(out) = output {
        out.to_path_buf()
    } else {
        let mut out = input.to_path_buf();
        out.set_extension("txtc"); // Txtcode Bytecode
        out
    };

    // Serialize bytecode
    let serialized_bytes = bincode::serialize(&bytecode)?;

    // Encrypt (if requested)
    if encrypt {
        let encryptor = BytecodeEncryptor::new();
        let encrypted = encryptor.encrypt(&serialized_bytes)?;
        fs::write(&output_path, encrypted.serialize())?;
        println!("Compiled and encrypted to: {}", output_path.display());
    } else {
        fs::write(&output_path, &serialized_bytes)?;
        println!("Compiled to: {}", output_path.display());
    }

    Ok(())
}
