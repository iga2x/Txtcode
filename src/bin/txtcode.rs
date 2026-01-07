use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;

// Binary must use txtcode:: to access library modules (they're separate crates in same package)
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::{VirtualMachine, Value};
use txtcode::lexer::TokenKind;
use txtcode::compiler::optimizer::{Optimizer, OptimizationLevel};
use txtcode::security::{Obfuscator, BytecodeEncryptor};
use txtcode::compiler::bytecode::BytecodeCompiler;
use txtcode::cli::package;

#[derive(ClapParser)]
#[command(name = "txtcode")]
#[command(about = "Txt-code Programming Language")]
#[command(version = "0.1.0")]
#[command(subcommand_required = false)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// File to execute (when no subcommand specified)
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a Txt-code program
    Run {
        /// Input file
        file: PathBuf,
    },
    /// Compile a Txt-code program
    Compile {
        /// Input file
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Target (native, wasm, bytecode)
        #[arg(short, long, default_value = "bytecode")]
        target: String,
        /// Optimization level (none, basic, aggressive)
        #[arg(short, long, default_value = "basic")]
        optimize: String,
        /// Enable obfuscation
        #[arg(long)]
        obfuscate: bool,
        /// Enable encryption
        #[arg(long)]
        encrypt: bool,
    },
    /// Format Txt-code source files
    Format {
        /// Input file(s)
        files: Vec<PathBuf>,
        /// Write changes to files
        #[arg(short, long)]
        write: bool,
    },
    /// Lint Txt-code source files
    Lint {
        /// Input file(s)
        files: Vec<PathBuf>,
    },
    /// Start interactive REPL
    Repl,
    /// Package management
    Package {
        #[command(subcommand)]
        command: PackageCommands,
    },
}

#[derive(Subcommand)]
pub enum PackageCommands {
    /// Initialize a new package
    Init {
        /// Package name
        name: String,
        /// Package version
        #[arg(default_value = "0.1.0")]
        version: String,
    },
    /// Install dependencies
    Install,
    /// Update dependencies
    Update,
    /// Add a dependency
    Add {
        /// Package name
        name: String,
        /// Package version
        version: String,
    },
}

pub fn main() {
    let cli = Cli::parse();

    match (&cli.command, &cli.file) {
        (None, None) => {
            // No args → Start REPL
            if let Err(e) = start_repl() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (None, Some(file)) => {
            // File provided, no subcommand → Run file
            if let Err(e) = run_file(file) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (Some(cmd), _) => {
            // Subcommand provided → Use existing subcommand logic
            match cmd {
                Commands::Run { file } => {
                    if let Err(e) = run_file(file) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Compile { file, output, target: _target, optimize, obfuscate, encrypt } => {
                    if let Err(e) = compile_file(file, output.as_ref(), optimize, *obfuscate, *encrypt) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Format { files, write } => {
                    if let Err(e) = format_files(files, *write) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Lint { files } => {
                    if let Err(e) = lint_files(files) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Repl => {
                    if let Err(e) = start_repl() {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Package { command } => {
                    if let Err(e) = handle_package_command(&command) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn run_file(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Check if file exists
    if !file.exists() {
        return Err(format!("Error: File '{}' not found", file.display()).into());
    }

    // Check file size (limit to 10MB to prevent memory issues)
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    let metadata = fs::metadata(file)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "Error: File '{}' is too large ({} bytes). Maximum allowed: {} bytes",
            file.display(),
            metadata.len(),
            MAX_FILE_SIZE
        ).into());
    }

    // Read file
    let source = fs::read_to_string(file)?;
    
    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()
        .map_err(|e| format!("Lex error: {}", e))?;
    
    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;
    
    // Interpret with safety limits
    let mut vm = VirtualMachine::new();
    vm.interpret(&program)
        .map_err(|e| format!("Runtime error: {}", e))?;
    
    Ok(())
}

fn compile_file(
    file: &PathBuf,
    output: Option<&PathBuf>,
    optimize: &str,
    obfuscate: bool,
    encrypt: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(file)?;
    
    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;
    
    // Optimize
    let opt_level = match optimize {
        "none" => OptimizationLevel::None,
        "basic" => OptimizationLevel::Basic,
        "aggressive" => OptimizationLevel::Aggressive,
        _ => OptimizationLevel::Basic,
    };
    let optimizer = Optimizer::new(opt_level);
    optimizer.optimize_ast(&mut program);
    
    // Obfuscate if requested
    if obfuscate {
        let mut obfuscator = Obfuscator::new();
        program = obfuscator.obfuscate(&program);
    }
    
    // Compile to bytecode
    let mut compiler = BytecodeCompiler::new();
    let bytecode_program = compiler.compile(&program);
    
    // Encrypt if requested
    if encrypt {
        let encryptor = BytecodeEncryptor::new();
        let encrypted = encryptor.encrypt(&bincode::serialize(&bytecode_program)?)?;
        let output_path = output.map(|p| p.clone()).unwrap_or_else(|| {
            file.with_extension("txtc.encrypted")
        });
        fs::write(&output_path, encrypted.serialize())?;
        println!("Compiled and encrypted to: {}", output_path.display());
    } else {
        let output_path = output.map(|p| p.clone()).unwrap_or_else(|| {
            file.with_extension("txtc")
        });
        let serialized = bincode::serialize(&bytecode_program)?;
        fs::write(&output_path, serialized)?;
        println!("Compiled to: {}", output_path.display());
    }
    
    Ok(())
}

fn format_files(files: &[PathBuf], write: bool) -> Result<(), Box<dyn std::error::Error>> {
    for file in files {
        let source = fs::read_to_string(file)?;
        let formatted = txtcode::tools::formatter::Formatter::format_source(&source)?;
        
        if write {
            fs::write(file, formatted)?;
            println!("Formatted: {}", file.display());
        } else {
            print!("{}", formatted);
        }
    }
    Ok(())
}

fn lint_files(files: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = 0;
    for file in files {
        let source = fs::read_to_string(file)?;
        let issues = txtcode::tools::linter::Linter::lint_source(&source)?;
        
        if !issues.is_empty() {
            println!("{}:", file.display());
            for issue in issues {
                println!("  {}:{}: {}", issue.line, issue.column, issue.message);
                errors += 1;
            }
        }
    }
    
    if errors > 0 {
        eprintln!("\nFound {} linting issue(s)", errors);
        std::process::exit(1);
    } else {
        println!("No linting issues found");
    }
    
    Ok(())
}

fn start_repl() -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::DefaultEditor;
    
    let mut rl = DefaultEditor::new()?;
    println!("Txt-code REPL v0.1.0");
    println!("Type 'exit' or 'quit' to quit, 'help' for help");
    
    let mut vm = VirtualMachine::new();
    
    loop {
        let readline = rl.readline("txtcode> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                
                if trimmed == "exit" || trimmed == "quit" {
                    break;
                }
                
                if trimmed == "help" {
                    println!("Commands:");
                    println!("  exit, quit  - Exit the REPL");
                    println!("  help        - Show this help");
                    println!("  clear       - Clear the screen");
                    continue;
                }
                
                if trimmed == "clear" {
                    print!("\x1B[2J\x1B[1;1H");
                    continue;
                }
                
                if trimmed.is_empty() {
                    continue;
                }
                
                // Try to evaluate
                let mut lexer = Lexer::new(trimmed.to_string());
                match lexer.tokenize() {
                    Ok(tokens) => {
                        // Explicit type annotation for type inference
                        let tokens: Vec<txtcode::lexer::Token> = tokens;
                        if tokens.is_empty() {
                            continue;
                        }
                        if let Some(last_token) = tokens.last() {
                            if last_token.kind == TokenKind::Eof {
                                continue;
                            }
                        }
                        
                        let mut parser = Parser::new(tokens);
                        match parser.parse() {
                            Ok(program) => {
                                match vm.interpret(&program) {
                                    Ok(value) => {
                                        if !matches!(value, Value::Null) {
                                            println!("{}", Value::to_string(&value));
                                        }
                                    }
                                    Err(e) => eprintln!("Runtime error: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Parse error: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Lex error: {}", e),
                }
            }
            Err(_) => break,
        }
    }
    
    Ok(())
}

fn handle_package_command(command: &PackageCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PackageCommands::Init { name, version } => {
            package::init_package(name.clone(), version.clone())?;
        }
        PackageCommands::Install => {
            package::install_dependencies()?;
        }
        PackageCommands::Update => {
            package::update_dependencies()?;
        }
        PackageCommands::Add { name, version } => {
            package::add_dependency(name.clone(), version.clone())?;
        }
    }
    Ok(())
}

