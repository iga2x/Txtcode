use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;

// Binary must use txtcode:: to access library modules (they're separate crates in same package)
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;
use txtcode::runtime::Value;
use txtcode::lexer::TokenKind;
use txtcode::compiler::optimizer::{Optimizer, OptimizationLevel};
use txtcode::security::{Obfuscator, BytecodeEncryptor};
use txtcode::compiler::bytecode::BytecodeCompiler;
use txtcode::cli::package;
use txtcode::config::Config;
use txtcode::tools::logger;
use sha2::{Sha256, Digest};

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
    
    /// Enable safe mode (disables exec() function)
    #[arg(long, global = true)]
    pub safe_mode: bool,
    
    /// Allow exec() function (overrides --safe-mode)
    #[arg(long, global = true)]
    pub allow_exec: bool,
    
    /// Enable debug mode (verbose execution logging)
    #[arg(long, short = 'd', global = true)]
    pub debug: bool,
    
    /// Enable verbose output
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,
    
    /// Quiet mode (minimal output)
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,
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
                #[arg(long, default_value = "basic")]
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
    /// Migrate code between versions
    Migrate {
        /// Files to migrate
        #[arg(short, long)]
        files: Vec<String>,
        /// Directory to migrate (all .tc files)
        #[arg(short, long)]
        directory: Option<String>,
        /// Source version (e.g., "0.1.0")
        #[arg(long)]
        from: Option<String>,
        /// Target version (e.g., "0.2.0")
        #[arg(long)]
        to: Option<String>,
        /// Dry run (don't modify files, just report)
        #[arg(long, default_value = "true")]
        dry_run: bool,
        /// Strict mode (errors on deprecated features)
        #[arg(long)]
        strict: bool,
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
    /// List dependencies
    List,
}

pub fn main() {
    // Initialize txtcode runtime directories
    if let Err(e) = txtcode::config::Config::ensure_directories() {
        eprintln!("Warning: Failed to initialize txtcode directories: {}", e);
    }
    
    // Initialize default config if it doesn't exist
    if let Err(e) = txtcode::config::Config::init_default_config() {
        eprintln!("Warning: Failed to initialize config file: {}", e);
    }

    // Initialize logger
    if let Err(e) = txtcode::tools::logger::init_logger("txtcode") {
        eprintln!("Warning: Failed to initialize logger: {}", e);
    }

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
            if let Err(e) = run_file(file, cli.safe_mode, cli.allow_exec, cli.debug, cli.verbose) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (Some(cmd), _) => {
            // Subcommand provided → Use existing subcommand logic
            match cmd {
                Commands::Run { file } => {
                    if let Err(e) = run_file(file, cli.safe_mode, cli.allow_exec, cli.debug, cli.verbose) {
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
                Commands::Migrate { files, directory, from, to, dry_run, strict } => {
                    use txtcode::cli::migrate::migrate_command;
                    if let Err(e) = migrate_command(files.clone(), from.clone(), to.clone(), *dry_run, *strict, directory.clone()) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn run_file(file: &PathBuf, safe_mode: bool, allow_exec: bool, debug: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Load user config for defaults
    let user_config = Config::load_config().unwrap_or_default();
    
    // Use config defaults if CLI args not provided (CLI args override config)
    let safe_mode = safe_mode || user_config.runtime.safe_mode;
    let allow_exec = allow_exec || user_config.runtime.allow_exec;
    let debug = debug || user_config.runtime.debug;
    let verbose = verbose || user_config.runtime.verbose;
    
    logger::log_info(&format!("Running file: {}", file.display()));
    
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
    
    // Interpret with safety limits and options
    let exec_allowed = if allow_exec { true } else { !safe_mode };
    let mut vm = VirtualMachine::with_all_options(safe_mode, debug, verbose);
    vm.set_exec_allowed(exec_allowed);
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
    // Load user config for defaults
    let user_config = Config::load_config().unwrap_or_default();
    
    // Use config defaults if CLI args not provided (use CLI args as override)
    let optimize = if optimize == "basic" && user_config.compiler.optimization != "basic" {
        &user_config.compiler.optimization
    } else {
        optimize
    };
    let obfuscate = obfuscate || user_config.compiler.obfuscate;
    let encrypt = encrypt || user_config.compiler.encrypt;
    
    let source = fs::read_to_string(file)?;
    
    // Check cache if enabled
    if user_config.package.cache_packages && !encrypt {
        let cache_key = generate_cache_key(&source, optimize, obfuscate)?;
        let cache_path = Config::get_cache_path(&cache_key)?;
        
        if cache_path.exists() {
            logger::log_info(&format!("Using cached bytecode for: {}", file.display()));
            
            // If output specified, copy from cache; otherwise use cache directly
            if let Some(output_path) = output {
                fs::copy(&cache_path, output_path)?;
                println!("Compiled (from cache) to: {}", output_path.display());
            } else {
                let default_output = file.with_extension("txtc");
                fs::copy(&cache_path, &default_output)?;
                println!("Compiled (from cache) to: {}", default_output.display());
            }
            return Ok(());
        }
    }
    
    logger::log_info(&format!("Compiling: {}", file.display()));
    
    // Lex
    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse()?;
    
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
        logger::log_info(&format!("Compiled and encrypted: {}", output_path.display()));
    } else {
        let serialized = bincode::serialize(&bytecode_program)?;
        
        // Save to cache if enabled
        if user_config.package.cache_packages {
            let cache_key = generate_cache_key(&source, optimize, obfuscate)?;
            let cache_path = Config::get_cache_path(&cache_key)?;
            
            // Ensure cache directory exists
            if let Some(parent) = cache_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            fs::write(&cache_path, &serialized)?;
            logger::log_info(&format!("Cached bytecode: {}", cache_path.display()));
        }
        
        let output_path = output.map(|p| p.clone()).unwrap_or_else(|| {
            file.with_extension("txtc")
        });
        fs::write(&output_path, serialized)?;
        println!("Compiled to: {}", output_path.display());
        logger::log_info(&format!("Compiled: {}", output_path.display()));
    }
    
    Ok(())
}

/// Generate a cache key from source code and compile options
fn generate_cache_key(source: &str, optimize: &str, obfuscate: bool) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(optimize.as_bytes());
    hasher.update(if obfuscate { b"1" } else { b"0" });
    let hash = hasher.finalize();
    Ok(hex::encode(&hash[..16])) // Use first 16 bytes for shorter key
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
        PackageCommands::List => {
            package::list_dependencies()?;
        }
    }
    Ok(())
}

