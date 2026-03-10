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
use txtcode::cli::env as env_cli;
use txtcode::cli::self_manage;
use txtcode::config::Config;
use txtcode::runtime::permissions::PermissionResource;
use txtcode::tools::logger;
use txtcode::tools::debugger::Debugger;
use sha2::{Sha256, Digest};

#[derive(ClapParser)]
#[command(name = "txtcode")]
#[command(about = "Txt-code Programming Language")]
#[command(version = "0.3.0")]
#[command(subcommand_required = false)]
#[command(after_help = "Examples:\n  txtcode                         # Start REPL\n  txtcode script.tc               # Run a file\n  txtcode run script.tc           # Run explicitly\n  txtcode format src/ --write     # Format files in-place\n  txtcode lint src/               # Lint a directory\n  txtcode compile main.tc -o app  # Compile to bytecode\n  txtcode test                    # Run tests in ./tests\n  txtcode doc --format html       # Generate docs\n  txtcode bench benches/fib.txt   # Benchmark a program\n  txtcode doctor                  # Check environment\n  txtcode init my-project         # Initialize a new project")]
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
        /// Maximum execution time (e.g. 30s, 500ms, 2m)
        #[arg(long)]
        timeout: Option<String>,
        /// Enable sandbox mode — deny all filesystem writes, network, and exec
        #[arg(long)]
        sandbox: bool,
        /// Load environment variables from a .env file before running
        #[arg(long, value_name = "FILE")]
        env_file: Option<PathBuf>,
        /// Disable coloured output (also honoured via NO_COLOR env var)
        #[arg(long)]
        no_color: bool,
        /// Output errors as JSON (useful for CI / AI consumers)
        #[arg(long)]
        json: bool,
        /// Advisory memory limit (e.g. 512mb, 1gb) — logged, not enforced
        #[arg(long, value_name = "LIMIT")]
        memory: Option<String>,
        /// Allow filesystem access scoped to a path (e.g. --allow-fs=/tmp).
        /// Can be specified multiple times. Overrides --sandbox for matched paths.
        #[arg(long, value_name = "PATH")]
        allow_fs: Vec<String>,
        /// Allow network access scoped to a host pattern (e.g. --allow-net=api.example.com).
        /// Can be specified multiple times. Overrides --sandbox for matched hosts.
        #[arg(long, value_name = "HOST")]
        allow_net: Vec<String>,
    },
    /// Inspect / disassemble a compiled bytecode file
    Inspect {
        /// Compiled bytecode file (.tcc or .bin)
        file: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
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
        /// Write changes to files in-place
        #[arg(short, long)]
        write: bool,
        /// CI mode: exit non-zero if any file needs formatting (do not write)
        #[arg(long)]
        check: bool,
    },
    /// Lint Txt-code source files
    Lint {
        /// Input file(s)
        files: Vec<PathBuf>,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
        /// Auto-fix fixable issues in-place (trailing whitespace, blank lines)
        #[arg(long)]
        fix: bool,
    },
    /// Start interactive REPL
    Repl,
    /// Package management
    Package {
        #[command(subcommand)]
        command: PackageCommands,
    },
    /// Debug a Txt-code program interactively
    Debug {
        /// File to debug
        file: PathBuf,
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
    /// Initialize a new Txt-code project
    Init {
        /// Project name (defaults to current directory name)
        name: Option<String>,
    },
    /// Check environment health
    Doctor,
    /// Run tests
    Test {
        /// Path to test directory or file (default: tests/)
        #[arg(default_value = "tests")]
        path: PathBuf,
        /// Only run tests whose filename contains this string
        #[arg(short, long)]
        filter: Option<String>,
        /// Watch mode: re-run tests automatically when source files change
        #[arg(short, long)]
        watch: bool,
    },
    /// Generate documentation from source files
    Doc {
        /// Input file(s)
        files: Vec<PathBuf>,
        /// Output directory (default: docs/api/)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format: markdown or html
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Benchmark a Txt-code program
    Bench {
        /// File to benchmark
        file: PathBuf,
        /// Number of runs (warmup excluded)
        #[arg(short, long, default_value = "20")]
        runs: usize,
        /// Warmup runs (not counted)
        #[arg(long, default_value = "3")]
        warmup: usize,
        /// Save results to a JSON file for later comparison
        #[arg(long, value_name = "FILE")]
        save: Option<PathBuf>,
        /// Compare current run against a previously saved results file
        #[arg(long, value_name = "FILE")]
        compare: Option<PathBuf>,
    },
    /// Virtual environment management (project-local packages + permissions)
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
    /// Manage the Txt-code installation itself (update, uninstall, info)
    #[command(name = "self")]
    SelfManage {
        #[command(subcommand)]
        command: SelfCommands,
    },
}

#[derive(Subcommand)]
pub enum EnvCommands {
    /// Create a new named environment (default: dev)
    Init {
        /// Environment name (dev, prod, test, sandbox, or custom)
        name: Option<String>,
        /// Use sandbox preset (deny all network + exec)
        #[arg(long)]
        sandbox: bool,
        /// Create all standard presets (dev, prod, test, sandbox)
        #[arg(long)]
        all: bool,
    },
    /// Install dependencies listed in Txtcode.toml into the active env
    Install,
    /// Switch the active named environment
    Use {
        /// Environment name to activate
        name: String,
    },
    /// Show active environment status and loaded config
    Status,
    /// List all named environments in this project
    List,
    /// Remove all installed packages from the active (or named) env
    Clean {
        /// Environment name (defaults to active)
        name: Option<String>,
    },
    /// Remove a named environment entirely
    Remove {
        /// Environment name (defaults to active)
        name: Option<String>,
    },
    /// Run health checks on the active environment
    Doctor,
    /// Show permission diff between two named environments
    Diff {
        /// First environment name
        a: String,
        /// Second environment name
        b: String,
    },
    /// Print current environment config as a TOML snapshot
    Freeze,
    /// Output a shell function for showing the active env in your prompt
    ShellHook,
    /// Print the path to the current environment directory
    Path,
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
    /// Remove a dependency from Txtcode.toml and uninstall it
    Remove {
        /// Package name to remove
        name: String,
    },
    /// Search for packages by name or keyword
    Search {
        /// Search query
        query: String,
    },
    /// Show detailed info about a package
    Info {
        /// Package name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum SelfCommands {
    /// Update Txt-code to the latest version
    Update,
    /// Uninstall Txt-code from this system
    Uninstall {
        /// Skip confirmation prompts (defaults to binary-only removal)
        #[arg(short, long)]
        yes: bool,
    },
    /// Show installation info (binary path, data size, project envs)
    Info,
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

    // Merge CLI flags with config defaults for runtime-related options
    let user_config = Config::load_config().unwrap_or_default();
    let safe_mode = cli.safe_mode || user_config.runtime.safe_mode;
    let allow_exec = cli.allow_exec || user_config.runtime.allow_exec;
    let debug = cli.debug || user_config.runtime.debug;
    let verbose = cli.verbose || user_config.runtime.verbose;
    let quiet = cli.quiet;

    // Special case: only flags, no command or file
    // - `txtcode -v` → verbose version/info and exit
    if cli.command.is_none() && cli.file.is_none() {
        if cli.verbose {
            print_verbose_info();
            return;
        }
    }

    match (&cli.command, &cli.file) {
        (None, None) => {
            // No args → Start REPL (respecting global flags / config)
            if let Err(e) = start_repl(safe_mode, allow_exec, debug, verbose, quiet) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (None, Some(file)) => {
            // File provided, no subcommand → Run file
            if let Err(e) = run_file(file, safe_mode, allow_exec, debug, verbose) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (Some(cmd), _) => {
            // Subcommand provided → Use existing subcommand logic
            match cmd {
                Commands::Run { file, timeout, sandbox, env_file, no_color, json, memory, allow_fs, allow_net } => {
                    // NO_COLOR support
                    if *no_color || std::env::var_os("NO_COLOR").is_some() {
                        std::env::set_var("NO_COLOR", "1");
                    }
                    // Memory limit (advisory — logged only)
                    if let Some(mem) = memory {
                        logger::log_info(&format!("Memory limit (advisory): {}", mem));
                        if verbose {
                            eprintln!("Note: --memory {} is advisory; hard enforcement requires OS-level limits", mem);
                        }
                    }
                    // Load .env file if specified
                    if let Some(env_path) = env_file {
                        if let Err(e) = load_env_file(env_path) {
                            if *json {
                                eprintln!("{{\"error\":\"{}\",\"type\":\"EnvFileError\"}}", e);
                            } else {
                                eprintln!("Error loading env-file: {}", e);
                            }
                            std::process::exit(1);
                        }
                    }
                    let effective_safe = safe_mode || *sandbox;
                    let effective_allow_exec = if *sandbox { false } else { allow_exec };
                    let result = if let Some(ts) = timeout {
                        run_file_with_timeout(file, effective_safe, effective_allow_exec, debug, verbose, ts)
                    } else {
                        run_file_with_allowlists(file, effective_safe, effective_allow_exec, debug, verbose, allow_fs, allow_net)
                    };
                    if let Err(e) = result {
                        if *json {
                            let msg = e.to_string().replace('"', "\\\"");
                            eprintln!("{{\"error\":\"{}\",\"type\":\"RuntimeError\"}}", msg);
                        } else {
                            eprintln!("Error: {}", e);
                        }
                        std::process::exit(1);
                    }
                }
                Commands::Compile { file, output, target, optimize, obfuscate, encrypt } => {
                    match target.as_str() {
                        "bytecode" | "bc" => {}
                        "native" | "wasm" => {
                            eprintln!(
                                "Error: compile target '{}' is not yet supported. Only 'bytecode' is implemented in this release.",
                                target
                            );
                            std::process::exit(1);
                        }
                        other => {
                            eprintln!(
                                "Error: unknown compile target '{}'. Valid targets: bytecode",
                                other
                            );
                            std::process::exit(1);
                        }
                    }
                    if let Err(e) = compile_file(file, output.as_ref(), optimize, *obfuscate, *encrypt) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Inspect { file, format } => {
                    if let Err(e) = inspect_bytecode(file, format) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Format { files, write, check } => {
                    if let Err(e) = format_files(files, *write, *check) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Lint { files, format, fix } => {
                    if let Err(e) = lint_files(files, format, *fix) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Repl => {
                    if let Err(e) = start_repl(safe_mode, allow_exec, debug, verbose, quiet) {
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
                Commands::Debug { file } => {
                    if let Err(e) = start_debug_repl(file) {
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
                Commands::Init { name } => {
                    if let Err(e) = init_project(name.as_deref()) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Doctor => {
                    run_doctor();
                }
                Commands::Test { path, filter, watch } => {
                    if *watch {
                        if let Err(e) = run_tests_watch(path, filter.as_deref()) {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    } else if let Err(e) = run_tests(path, filter.as_deref()) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Doc { files, output, format } => {
                    if let Err(e) = generate_docs(files, output.as_ref(), format) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Bench { file, runs, warmup, save, compare } => {
                    if let Err(e) = benchmark_file(file, *runs, *warmup, save.as_ref(), compare.as_ref()) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Env { command } => {
                    if let Err(e) = handle_env_command(command) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::SelfManage { command } => {
                    if let Err(e) = handle_self_command(command) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn handle_env_command(command: &EnvCommands) -> Result<(), String> {
    match command {
        EnvCommands::Init { name, sandbox, all } => {
            env_cli::env_init(name.clone(), *sandbox, *all)
                .map_err(|e| e.to_string())
        }
        EnvCommands::Install => {
            env_cli::env_install().map_err(|e| e.to_string())
        }
        EnvCommands::Use { name } => {
            env_cli::env_use(name).map_err(|e| e.to_string())
        }
        EnvCommands::Status => {
            env_cli::env_status().map_err(|e| e.to_string())
        }
        EnvCommands::List => {
            env_cli::env_list().map_err(|e| e.to_string())
        }
        EnvCommands::Clean { name } => {
            env_cli::env_clean(name.clone()).map_err(|e| e.to_string())
        }
        EnvCommands::Remove { name } => {
            env_cli::env_remove(name.clone()).map_err(|e| e.to_string())
        }
        EnvCommands::Doctor => {
            env_cli::env_doctor().map_err(|e| e.to_string())
        }
        EnvCommands::Diff { a, b } => {
            env_cli::env_diff(a, b).map_err(|e| e.to_string())
        }
        EnvCommands::Freeze => {
            env_cli::env_freeze().map_err(|e| e.to_string())
        }
        EnvCommands::ShellHook => {
            env_cli::env_shell_hook().map_err(|e| e.to_string())
        }
        EnvCommands::Path => {
            env_cli::env_path().map_err(|e| e.to_string())
        }
    }
}

/// Run a file with optional filesystem/network path allowlists.
/// `allow_fs` entries add scoped `fs.write`/`fs.read` grants for the given paths.
/// `allow_net` entries add scoped `net.connect` grants for the given host patterns.
fn run_file_with_allowlists(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if allow_fs.is_empty() && allow_net.is_empty() {
        return run_file(file, safe_mode, allow_exec, debug, verbose);
    }
    // We need to wire the extra grants into the VirtualMachine before running.
    // Do that by calling run_file_inner directly.
    run_file_inner(file, safe_mode, allow_exec, debug, verbose, allow_fs, allow_net)
}

fn run_file(file: &PathBuf, safe_mode: bool, allow_exec: bool, debug: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    run_file_inner(file, safe_mode, allow_exec, debug, verbose, &[], &[])
}

fn run_file_inner(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    allow_fs: &[String],
    allow_net: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    logger::log_info(&format!("Running file: {}", file.display()));

    // Directory: suggest txtcode test or list tc files
    if file.is_dir() {
        return Err(format!(
            "'{}' is a directory, not a file.\n  To run tests: txtcode test {}\n  To run a file: txtcode src/main.tc",
            file.display(),
            file.display()
        ).into());
    }

    // Check if file exists
    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }

    // Extension check: catch accidental .rs / .py / .js files
    if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
        match ext {
            "tc" => {} // ok
            "txt" => {
                return Err(format!(
                    "'{}' has a .txt extension which is a plain text file.\n  Txt-code source files use the .tc extension.",
                    file.display()
                ).into());
            }
            "rs" | "py" | "js" | "ts" | "go" | "rb" | "java" | "c" | "cpp" => {
                return Err(format!(
                    "'{}' is a {} file, not a Txt-code file.\n  Txt-code source files use the .tc extension.",
                    file.display(),
                    ext
                ).into());
            }
            _ => {} // allow unknown extensions (user's choice)
        }
    }

    // Check file size (limit to 10MB to prevent memory issues)
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    let metadata = fs::metadata(file)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File '{}' is too large ({} bytes, max 10MB)",
            file.display(),
            metadata.len()
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
    
    // Resolve effective safe_mode: CLI flag OR env profile safe_mode
    let env_safe_mode = Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false);
    let effective_safe_mode = safe_mode || env_safe_mode;
    let exec_allowed = if allow_exec { true } else { !effective_safe_mode };

    let mut vm = VirtualMachine::with_all_options(effective_safe_mode, debug, verbose);
    vm.set_exec_allowed(exec_allowed);

    // Apply project env permissions (allow/deny lists from env.toml)
    apply_env_permissions(&mut vm);

    // Apply CLI --allow-fs / --allow-net path allowlists
    apply_cli_allowlists(&mut vm, allow_fs, allow_net);

    vm.interpret(&program)
        .map_err(|e| format!("Runtime error: {}", e))?;

    Ok(())
}

/// Parse a permission string like "fs.read", "net.connect", "process.exec", "sys.getenv"
/// into a PermissionResource.
fn parse_permission_string(s: &str) -> Option<PermissionResource> {
    let (prefix, action) = s.split_once('.')?;
    match prefix {
        "fs" | "filesystem" => Some(PermissionResource::FileSystem(action.to_string())),
        "net" | "network"   => Some(PermissionResource::Network(action.to_string())),
        "process" | "proc"  => Some(PermissionResource::Process(vec![action.to_string()])),
        "sys" | "system"    => Some(PermissionResource::System(action.to_string())),
        _ => None,
    }
}

/// Load the active env's allow/deny permission lists and apply them to the VM.
/// Called by run_file and start_repl so that project-level env.toml is enforced.
fn apply_env_permissions(vm: &mut VirtualMachine) {
    if let Some((_env_dir, _name, cfg)) = Config::load_active_env() {
        for perm_str in &cfg.permissions.allow {
            if let Some(resource) = parse_permission_string(perm_str) {
                vm.grant_permission(resource, None);
            }
        }
        for perm_str in &cfg.permissions.deny {
            if let Some(resource) = parse_permission_string(perm_str) {
                vm.deny_permission(resource, None);
            }
        }
    }
}

/// Grant scoped permissions from CLI --allow-fs / --allow-net flags.
///
/// --allow-fs=/tmp  → grants fs.read + fs.write with scope "/tmp/*"
/// --allow-net=api.example.com → grants net.connect with scope "api.example.com"
fn apply_cli_allowlists(vm: &mut VirtualMachine, allow_fs: &[String], allow_net: &[String]) {
    use txtcode::runtime::permissions::PermissionResource;
    for path in allow_fs {
        // Normalise: "/tmp" -> "/tmp/*", "/tmp/" -> "/tmp/*"
        let scope = if path.ends_with('/') || path.ends_with('*') {
            format!("{}*", path.trim_end_matches(['/', '*']))
        } else {
            format!("{}/*", path)
        };
        vm.grant_permission(PermissionResource::FileSystem("read".to_string()), Some(scope.clone()));
        vm.grant_permission(PermissionResource::FileSystem("write".to_string()), Some(scope));
    }
    for host in allow_net {
        vm.grant_permission(PermissionResource::Network("connect".to_string()), Some(host.clone()));
    }
}

fn inspect_bytecode(file: &PathBuf, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    use txtcode::compiler::bytecode::Bytecode;
    let bytes = std::fs::read(file)?;
    let bytecode: Bytecode = bincode::deserialize(&bytes)
        .map_err(|e| format!("Failed to deserialize bytecode: {}. Is this a compiled .tcc file?", e))?;
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

fn format_files(files: &[PathBuf], write: bool, check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut needs_format: Vec<PathBuf> = Vec::new();
    for file in files {
        let source = fs::read_to_string(file)?;
        let formatted = txtcode::tools::formatter::Formatter::format_source(&source)?;

        if check {
            if source != formatted {
                println!("needs formatting: {}", file.display());
                needs_format.push(file.clone());
            }
        } else if write {
            if source != formatted {
                fs::write(file, &formatted)?;
                println!("  formatted  {}", file.display());
            }
        } else {
            print!("{}", formatted);
        }
    }
    if check && !needs_format.is_empty() {
        eprintln!("\n{} file(s) need formatting. Run with --write to fix.", needs_format.len());
        std::process::exit(1);
    }
    Ok(())
}

fn lint_files(files: &[PathBuf], format: &str, fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    use txtcode::tools::linter::{Linter, Severity};
    let mut error_count = 0usize;
    let mut warning_count = 0usize;
    let json_out = format == "json";
    let mut json_issues: Vec<String> = Vec::new();

    for file in files {
        let source = fs::read_to_string(file)?;
        let issues = Linter::lint_source_with_path(&source, Some(file.as_path()))?;

        // Auto-fix mode: fix trailing whitespace and excessive blank lines
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
                    issue.line, issue.column,
                    issue.severity,
                    issue.message.replace('"', "\\\"")
                ));
            } else {
                let prefix = match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                };
                println!("  [{}] {}:{}:{} — {}", prefix, file.display(), issue.line, issue.column, issue.message);
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

fn start_repl(
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::{DefaultEditor, error::ReadlineError};
    
    let mut rl = DefaultEditor::new()?;

    if !quiet {
        println!("Txt-code REPL v{}", env!("CARGO_PKG_VERSION"));
        println!("Type 'exit' or 'quit' to quit, 'help' for help");
    }
    
    let env_safe_mode = Config::load_active_env()
        .map(|(_, _, cfg)| cfg.permissions.safe_mode)
        .unwrap_or(false);
    let effective_safe_mode = safe_mode || env_safe_mode;
    let exec_allowed = if allow_exec { true } else { !effective_safe_mode };
    let mut vm = VirtualMachine::with_all_options(effective_safe_mode, debug, verbose);
    vm.set_exec_allowed(exec_allowed);
    apply_env_permissions(&mut vm);

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
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                // Ctrl+C or Ctrl+D — exit REPL gracefully
                break;
            }
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

fn print_verbose_info() {
    let version = env!("CARGO_PKG_VERSION");
    let build = if cfg!(debug_assertions) { "debug" } else { "release" };
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!("Txt-code v{}", version);
    println!("Build: {}", build);
    println!("Platform: {}-{}", os, arch);
}

fn start_debug_repl(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use rustyline::DefaultEditor;

    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }

    let source = fs::read_to_string(file)?;

    // Compile to bytecode
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    let total = bytecode.instructions.len();
    let mut debugger = Debugger::new();
    debugger.load(bytecode);

    println!("Txt-code Debugger — {} ({} instructions)", file.display(), total);
    println!("Commands: step/s, continue/c, break/b <n>, inspect/i <var>, stack, vars, quit/q, help");
    println!("ip=0 ready");

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("(debug) ");
        let line = match readline {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(trimmed);
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        match parts[0] {
            "step" | "s" => {
                match debugger.step() {
                    Ok(state) => {
                        if state.done {
                            println!("Program finished.");
                        } else {
                            println!("ip={} | {}", state.ip, state.instruction);
                        }
                    }
                    Err(e) => eprintln!("Step error: {}", e),
                }
            }
            "continue" | "c" => {
                match debugger.continue_execution() {
                    Ok(_) => println!("Execution complete."),
                    Err(e) => println!("{}", e),
                }
            }
            "break" | "b" => {
                if let Some(addr_str) = parts.get(1) {
                    if let Ok(addr) = addr_str.trim().parse::<usize>() {
                        debugger.add_breakpoint(addr);
                        println!("Breakpoint set at ip={}", addr);
                    } else {
                        eprintln!("Usage: break <instruction_index>");
                    }
                } else {
                    let bps = debugger.list_breakpoints();
                    if bps.is_empty() {
                        println!("No breakpoints set.");
                    } else {
                        println!("Breakpoints: {:?}", bps);
                    }
                }
            }
            "inspect" | "i" => {
                if let Some(name) = parts.get(1) {
                    match debugger.inspect_variable(name.trim()) {
                        Some(val) => println!("{} = {:?}", name.trim(), val),
                        None => println!("Variable '{}' not found", name.trim()),
                    }
                } else {
                    eprintln!("Usage: inspect <variable>");
                }
            }
            "stack" => {
                let stack = debugger.get_stack();
                if stack.is_empty() {
                    println!("Stack: (empty)");
                } else {
                    println!("Stack ({} items):", stack.len());
                    for (i, val) in stack.iter().enumerate().rev() {
                        println!("  [{}] {:?}", i, val);
                    }
                }
            }
            "vars" => {
                let vars = debugger.get_all_variables();
                if vars.is_empty() {
                    println!("No variables defined.");
                } else {
                    println!("Variables ({}):", vars.len());
                    for (k, v) in &vars {
                        println!("  {} = {:?}", k, v);
                    }
                }
            }
            "callstack" => {
                let frames = debugger.get_call_stack();
                if frames.is_empty() {
                    println!("Call stack: (empty)");
                } else {
                    for frame in &frames {
                        println!("  {}", frame);
                    }
                }
            }
            "help" | "?" => {
                println!("Commands:");
                println!("  step / s               — execute one instruction");
                println!("  continue / c           — run until breakpoint or end");
                println!("  break / b <n>          — set breakpoint at instruction n");
                println!("  break / b              — list breakpoints");
                println!("  inspect / i <var>      — inspect variable value");
                println!("  stack                  — show operand stack");
                println!("  vars                   — show all variables");
                println!("  callstack              — show call stack frames");
                println!("  quit / q               — exit debugger");
            }
            "quit" | "q" => break,
            _ => eprintln!("Unknown command '{}'. Type 'help' for commands.", parts[0]),
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
        PackageCommands::Remove { name } => {
            package::remove_dependency(name.clone())?;
        }
        PackageCommands::Search { query } => {
            package_search(query)?;
        }
        PackageCommands::Info { name } => {
            package_info(name)?;
        }
    }
    Ok(())
}

fn handle_self_command(command: &SelfCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        SelfCommands::Update => {
            self_manage::self_update()?;
        }
        SelfCommands::Uninstall { yes } => {
            self_manage::self_uninstall(*yes)?;
        }
        SelfCommands::Info => {
            self_manage::self_info()?;
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode init
// ────────────────────────────────────────────────────────────────────────────

fn init_project(name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let project_name = name.map(|n| n.to_string()).unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string()
    });

    // Decide whether to create a subdirectory or use cwd
    let project_dir = if let Some(n) = name {
        let d = cwd.join(n);
        fs::create_dir_all(&d)?;
        d
    } else {
        cwd.clone()
    };

    // Txtcode.toml
    let toml_path = project_dir.join("Txtcode.toml");
    if !toml_path.exists() {
        fs::write(
            &toml_path,
            format!(
                r#"name = "{name}"
version = "0.1.0"
description = "A new Txt-code project"
authors = []

[dependencies]
"#,
                name = project_name
            ),
        )?;
        println!("  created  Txtcode.toml");
    }

    // src/main.tc
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    let main_tc = src_dir.join("main.tc");
    if !main_tc.exists() {
        fs::write(
            &main_tc,
            format!(
                r#"## {name} — entry point
##
## Run with: txtcode src/main.tc

print → "Hello from {name}!"
"#,
                name = project_name
            ),
        )?;
        println!("  created  src/main.tc");
    }

    // tests/ directory with a sample test
    let tests_dir = project_dir.join("tests");
    fs::create_dir_all(&tests_dir)?;
    let sample_test = tests_dir.join("test_main.tc");
    if !sample_test.exists() {
        fs::write(
            &sample_test,
            r#"## Basic sanity test — runs automatically with: txtcode test

store → result → 1 + 1
assert → result == 2, "1 + 1 should equal 2"
print → "Tests passed"
"#,
        )?;
        println!("  created  tests/test_main.tc");
    }

    // .gitignore
    let gitignore = project_dir.join(".gitignore");
    if !gitignore.exists() {
        fs::write(
            &gitignore,
            r#"# Compiled bytecode
*.txtc
*.txtc.encrypted

# Package cache
.txtcode/

# Lock file (commit this to pin versions)
# Txtcode.lock

# Editor directories
.vscode/
.idea/
*.swp
"#,
        )?;
        println!("  created  .gitignore");
    }

    // README.md
    let readme = project_dir.join("README.md");
    if !readme.exists() {
        fs::write(
            &readme,
            format!(
                r#"# {name}

A [Txt-code](https://github.com/iga2x/txtcode) project.

## Getting started

```bash
# Run the main program
txtcode src/main.tc

# Run tests
txtcode test

# Format all source files
txtcode fmt --write src/

# Lint
txtcode lint src/
```

## Project layout

```
{name}/
├── Txtcode.toml   # Project manifest
├── src/
│   └── main.tc    # Entry point
└── tests/
    └── test_main.tc
```
"#,
                name = project_name
            ),
        )?;
        println!("  created  README.md");
    }

    println!("\nProject '{}' initialized.", project_name);
    if name.is_some() {
        println!("  cd {}", project_name);
    }
    println!("  txtcode src/main.tc");
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode doctor
// ────────────────────────────────────────────────────────────────────────────

fn run_doctor() {
    let mut ok = true;

    // Helper closures
    let pass = |msg: &str| println!("  [ok]   {}", msg);
    let fail = |msg: &str| {
        println!("  [FAIL] {}", msg);
    };
    let warn = |msg: &str| println!("  [warn] {}", msg);

    println!("txtcode doctor — environment check\n");

    // ── Binary / version ────────────────────────────────────────────────────
    match std::env::current_exe() {
        Ok(path) => pass(&format!("txtcode binary: {}", path.display())),
        Err(e) => {
            fail(&format!("Cannot determine binary path: {}", e));
            ok = false;
        }
    }
    pass("version: 0.1.0");

    // ── Project manifest ────────────────────────────────────────────────────
    let cwd = std::env::current_dir().unwrap_or_default();
    let manifest = cwd.join("Txtcode.toml");
    if manifest.exists() {
        pass(&format!("Project manifest found: {}", manifest.display()));
    } else {
        warn("No Txtcode.toml found in current directory (run `txtcode init` to create one)");
    }

    // ── src/ directory ──────────────────────────────────────────────────────
    let src_dir = cwd.join("src");
    if src_dir.exists() {
        // Check write permission
        match fs::metadata(&src_dir) {
            Ok(_) => pass(&format!("src/ directory: {}", src_dir.display())),
            Err(e) => {
                fail(&format!("src/ not accessible: {}", e));
                ok = false;
            }
        }
    } else {
        warn("No src/ directory in current project");
    }

    // ── ~/.txtcode directories ───────────────────────────────────────────────
    match txtcode::config::Config::get_txtcode_home() {
        Ok(home) => {
            if home.exists() {
                pass(&format!("txtcode home: {}", home.display()));
            } else {
                warn(&format!(
                    "txtcode home directory missing: {} (run any command to create it)",
                    home.display()
                ));
            }

            for subdir in &["cache", "packages", "logs"] {
                let path = home.join(subdir);
                if path.exists() {
                    pass(&format!("{}/: {}", subdir, path.display()));
                } else {
                    warn(&format!("{}/: not found (will be created on first use)", subdir));
                }
            }
        }
        Err(e) => {
            fail(&format!("Cannot resolve txtcode home: {}", e));
            ok = false;
        }
    }

    // ── Git availability ─────────────────────────────────────────────────────
    let git_ok = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if git_ok {
        pass("git: available");
    } else {
        warn("git: not found in PATH (install git for version control features)");
    }

    // ── Permissions: temp directory write ───────────────────────────────────
    let tmp = std::env::temp_dir();
    let probe = tmp.join("txtcode_doctor_probe");
    match fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            pass(&format!("temp directory writable: {}", tmp.display()));
        }
        Err(e) => {
            fail(&format!("temp directory not writable ({}): {}", tmp.display(), e));
            ok = false;
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    println!();
    if ok {
        println!("All checks passed.");
    } else {
        println!("Some checks failed — see [FAIL] items above.");
        std::process::exit(1);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode test
// ────────────────────────────────────────────────────────────────────────────

fn run_tests(path: &PathBuf, filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Collect test files: test_*.tc or *_test.tc under `path`
    let mut test_files: Vec<PathBuf> = Vec::new();

    if path.is_file() {
        test_files.push(path.clone());
    } else if path.is_dir() {
        collect_test_files(path, &mut test_files)?;
    } else {
        return Err(format!("Test path '{}' not found", path.display()).into());
    }

    // Apply filter
    if let Some(f) = filter {
        test_files.retain(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.contains(f))
        });
    }

    if test_files.is_empty() {
        println!("No test files found in '{}'.", path.display());
        println!("Test files must be named test_*.tc or *_test.tc");
        return Ok(());
    }

    println!("Running {} test file(s)...\n", test_files.len());

    let mut passed = 0usize;
    let mut failed = 0usize;

    for test_file in &test_files {
        let name = test_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let source = fs::read_to_string(test_file)?;
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                println!("  FAIL  {} — lex error: {}", name, e);
                failed += 1;
                continue;
            }
        };
        let mut parser = Parser::new(tokens);
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => {
                println!("  FAIL  {} — parse error: {}", name, e);
                failed += 1;
                continue;
            }
        };

        let mut vm = VirtualMachine::new();
        match vm.interpret(&program) {
            Ok(_) => {
                println!("  PASS  {}", name);
                passed += 1;
            }
            Err(e) => {
                println!("  FAIL  {} — {}", name, e);
                failed += 1;
            }
        }
    }

    println!("\n{} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn collect_test_files(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_files(&path, files)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if (name.starts_with("test_") || name.ends_with("_test.tc"))
                && name.ends_with(".tc")
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode doc
// ────────────────────────────────────────────────────────────────────────────

fn generate_docs(
    files: &[PathBuf],
    output: Option<&PathBuf>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use txtcode::tools::docgen::{DocGenerator, OutputFormat};

    let fmt = match format {
        "html" => OutputFormat::Html,
        _ => OutputFormat::Markdown,
    };
    let gen = DocGenerator::with_format(fmt);
    let ext = if format == "html" { "html" } else { "md" };

    // Default output directory
    let default_out = PathBuf::from("docs/api");
    let out_dir = output.unwrap_or(&default_out);
    fs::create_dir_all(out_dir)?;

    if files.is_empty() {
        // Auto-discover .tc files in src/
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

// ────────────────────────────────────────────────────────────────────────────
// txtcode bench
// ────────────────────────────────────────────────────────────────────────────

fn benchmark_file(
    file: &PathBuf,
    runs: usize,
    warmup: usize,
    save: Option<&PathBuf>,
    compare: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }
    if file.is_dir() {
        return Err(format!("'{}' is a directory", file.display()).into());
    }

    // Load previous results for comparison
    let prev: Option<(f64, f64, f64, f64)> = if let Some(cmp_path) = compare {
        match fs::read_to_string(cmp_path) {
            Ok(data) => parse_bench_json(&data),
            Err(e) => {
                eprintln!("Warning: could not read compare file '{}': {}", cmp_path.display(), e);
                None
            }
        }
    } else {
        None
    };

    // Compile once, reuse the program
    let source = fs::read_to_string(file)?;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    println!("Benchmarking: {}", file.display());
    println!("  Warmup: {} run(s), measured: {} run(s)\n", warmup, runs);

    // Warmup
    for _ in 0..warmup {
        let mut vm = VirtualMachine::new();
        vm.interpret(&program).map_err(|e| format!("Runtime error: {}", e))?;
    }

    // Timed runs (microseconds)
    let mut timings: Vec<f64> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = std::time::Instant::now();
        let mut vm = VirtualMachine::new();
        vm.interpret(&program).map_err(|e| format!("Runtime error: {}", e))?;
        timings.push(start.elapsed().as_micros() as f64);
    }

    let mean = timings.iter().sum::<f64>() / timings.len() as f64;
    let min = timings.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = timings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let variance =
        timings.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / timings.len() as f64;
    let stddev = variance.sqrt();

    let fmt_us = |us: f64| -> String {
        if us < 1000.0 {
            format!("{:.1}µs", us)
        } else if us < 1_000_000.0 {
            format!("{:.2}ms", us / 1000.0)
        } else {
            format!("{:.3}s", us / 1_000_000.0)
        }
    };

    println!("  Mean:    {}", fmt_us(mean));
    println!("  Min:     {}", fmt_us(min));
    println!("  Max:     {}", fmt_us(max));
    println!("  Std dev: {}", fmt_us(stddev));
    println!("  Runs:    {}", runs);

    // Comparison output
    if let Some((prev_mean, prev_min, prev_max, prev_stddev)) = prev {
        println!("\n  Comparison vs baseline:");
        let delta = |cur: f64, old: f64| {
            if old == 0.0 { return "n/a".to_string(); }
            let pct = (cur - old) / old * 100.0;
            if pct > 0.0 { format!("+{:.1}% slower", pct) }
            else { format!("{:.1}% faster", -pct) }
        };
        println!("  Mean:    {} → {} ({})", fmt_us(prev_mean), fmt_us(mean), delta(mean, prev_mean));
        println!("  Min:     {} → {} ({})", fmt_us(prev_min), fmt_us(min), delta(min, prev_min));
        println!("  Max:     {} → {} ({})", fmt_us(prev_max), fmt_us(max), delta(max, prev_max));
        println!("  Std dev: {} → {}", fmt_us(prev_stddev), fmt_us(stddev));
    }

    // Save results
    if let Some(save_path) = save {
        let json = format!(
            "{{\"mean_us\":{:.3},\"min_us\":{:.3},\"max_us\":{:.3},\"stddev_us\":{:.3},\"runs\":{},\"file\":\"{}\"}}",
            mean, min, max, stddev, runs, file.display()
        );
        fs::write(save_path, json)?;
        println!("\n  Results saved to {}", save_path.display());
    }

    Ok(())
}

/// Parse a minimal bench JSON: returns (mean, min, max, stddev) in microseconds.
fn parse_bench_json(data: &str) -> Option<(f64, f64, f64, f64)> {
    let get = |key: &str| -> Option<f64> {
        let needle = format!("\"{}\":", key);
        let pos = data.find(&needle)? + needle.len();
        let rest = data[pos..].trim_start();
        let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
        rest[..end].trim().parse().ok()
    };
    Some((get("mean_us")?, get("min_us")?, get("max_us")?, get("stddev_us")?))
}


// ────────────────────────────────────────────────────────────────────────────
// txtcode run --timeout
// ────────────────────────────────────────────────────────────────────────────

/// Parse duration strings like "30s", "500ms", "2m" into std::time::Duration.
fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        return ms.parse::<u64>().ok().map(std::time::Duration::from_millis);
    }
    if let Some(m) = s.strip_suffix('m') {
        return m.parse::<u64>().ok().map(|n| std::time::Duration::from_secs(n * 60));
    }
    if let Some(sec) = s.strip_suffix('s') {
        return sec.parse::<u64>().ok().map(std::time::Duration::from_secs);
    }
    s.parse::<u64>().ok().map(std::time::Duration::from_secs)
}

fn run_file_with_timeout(
    file: &PathBuf,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
    timeout_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let duration = parse_duration(timeout_str)
        .ok_or_else(|| format!("Invalid timeout format '{}'. Use e.g. 30s, 500ms, 2m", timeout_str))?;

    let file = file.clone();
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        let result = run_file(&file, safe_mode, allow_exec, debug, verbose)
            .map_err(|e| e.to_string());
        let _ = tx.send(result);
    });

    match rx.recv_timeout(duration) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("Execution timed out after {}", timeout_str).into())
        }
        Err(e) => Err(format!("Thread error: {}", e).into()),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode run --env-file
// ────────────────────────────────────────────────────────────────────────────

/// Parse a .env file (KEY=VALUE lines, # comments, blank lines ignored)
/// and set each key into the process environment.
fn load_env_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("env-file '{}' not found", path.display()).into());
    }
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let mut val = line[eq + 1..].trim();
            // Strip optional surrounding quotes
            if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                val = &val[1..val.len() - 1];
            }
            if !key.is_empty() {
                std::env::set_var(key, val);
            }
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode test --watch
// ────────────────────────────────────────────────────────────────────────────

fn run_tests_watch(
    path: &PathBuf,
    filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Watching for changes in '{}' (Ctrl+C to stop)...\n", path.display());

    // Snapshot: path -> last-modified
    let snapshot = |path: &PathBuf| -> std::collections::HashMap<PathBuf, std::time::SystemTime> {
        let mut map = std::collections::HashMap::new();
        let mut queue = vec![path.clone()];
        while let Some(dir) = queue.pop() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        queue.push(p);
                    } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if ext == "tc" {
                            if let Ok(meta) = fs::metadata(&p) {
                                if let Ok(modified) = meta.modified() {
                                    map.insert(p, modified);
                                }
                            }
                        }
                    }
                }
            }
        }
        map
    };

    let mut prev = snapshot(path);

    // Run once immediately
    let _ = run_tests(path, filter);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let current = snapshot(path);
        let changed = current.iter().any(|(p, t)| prev.get(p).map_or(true, |old| old != t))
            || prev.keys().any(|p| !current.contains_key(p));
        if changed {
            println!("\n── file change detected, re-running tests ──\n");
            let _ = run_tests(path, filter);
            prev = current;
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// txtcode package search / info
// ────────────────────────────────────────────────────────────────────────────

fn package_search(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Searching packages for '{}'...\n", query);

    // Check if a local registry is configured
    let registry_url = std::env::var("TXTCODE_REGISTRY").ok();

    if let Some(url) = registry_url {
        // Attempt to hit a registry endpoint
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent(format!("txtcode/{}", env!("CARGO_PKG_VERSION")))
                .build()?;
            let resp = client
                .get(format!("{}/search?q={}", url.trim_end_matches('/'), query))
                .send()
                .await?;
            resp.text().await
        });
        match result {
            Ok(body) => println!("{}", body),
            Err(e) => eprintln!("Registry error: {}", e),
        }
    } else {
        println!("No package registry configured.");
        println!("Set TXTCODE_REGISTRY=https://your-registry to enable search.");
        println!();
        println!("To add a known package manually:");
        println!("  txtcode package add <name> <version>");
    }
    Ok(())
}

fn package_info(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // First check if it's in the local Txtcode.toml
    let toml_path = PathBuf::from("Txtcode.toml");
    if toml_path.exists() {
        let content = fs::read_to_string(&toml_path)?;
        let in_local = content.lines().any(|l| {
            let t = l.trim();
            t.starts_with(name) && t.contains('=')
        });
        if in_local {
            println!("Package: {}", name);
            println!("Source:  local Txtcode.toml");
            // Extract version line
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with(name) && t.contains('=') {
                    println!("Entry:   {}", t);
                }
            }
            return Ok(());
        }
    }

    // Check registry
    let registry_url = std::env::var("TXTCODE_REGISTRY").ok();
    if let Some(url) = registry_url {
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent(format!("txtcode/{}", env!("CARGO_PKG_VERSION")))
                .build()?;
            let resp = client
                .get(format!("{}/packages/{}", url.trim_end_matches('/'), name))
                .send()
                .await?;
            resp.text().await
        });
        match result {
            Ok(body) => println!("{}", body),
            Err(e) => eprintln!("Registry error: {}", e),
        }
    } else {
        println!("Package '{}' not found in local Txtcode.toml.", name);
        println!("Set TXTCODE_REGISTRY=https://your-registry to look up remote packages.");
    }
    Ok(())
}
