use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;
use txtcode::cli::bench;
use txtcode::cli::check;
use txtcode::cli::compile;
use txtcode::cli::debug as debug_cli;
use txtcode::cli::doc;
use txtcode::cli::doctor;
use txtcode::cli::env as env_cli;
use txtcode::cli::format as format_cli;
use txtcode::cli::init;
use txtcode::cli::lint as lint_cli;
use txtcode::cli::package;
use txtcode::cli::repl as repl_cli;
use txtcode::cli::run as run_cli;
use txtcode::cli::self_manage;
use txtcode::cli::test_cmd;
use txtcode::config::Config;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::typecheck::TypeChecker;
use txtcode::validator::Validator;
use txtcode::runtime::vm::VirtualMachine;
use txtcode::runtime::Value;

#[derive(ClapParser)]
#[command(name = "txtcode")]
#[command(about = "Txt-code Programming Language")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(subcommand_required = false)]
#[command(
    after_help = "Examples:\n  txtcode                          # Start REPL\n  txtcode script.tc                # Run a file\n  txtcode -c \"print(1 + 1)\"        # Inline eval\n  echo 'print(42)' | txtcode -    # Stdin pipe\n  txtcode run script.tc --watch    # Re-run on file change\n  txtcode check src/               # Lint + type-check\n  txtcode format src/ --write      # Format in-place\n  txtcode lint src/                # Lint a directory\n  txtcode compile main.tc -o app   # Compile to bytecode\n  txtcode test --json              # JSON test results\n  txtcode env status --json        # Active env as JSON\n  txtcode bench benches/fib.txt    # Benchmark\n  txtcode init my-project          # New project"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// File to execute (when no subcommand given)
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Evaluate a snippet and exit  (like python -c)
    #[arg(short = 'c', value_name = "CODE", global = false)]
    pub eval: Option<String>,

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
        /// Watch mode: re-run when the file changes (Ctrl+C to stop)
        #[arg(long)]
        watch: bool,
        /// Allow filesystem access scoped to a path (e.g. --allow-fs=/tmp).
        #[arg(long, value_name = "PATH")]
        allow_fs: Vec<String>,
        /// Allow network access scoped to a host pattern (e.g. --allow-net=api.example.com).
        #[arg(long, value_name = "HOST")]
        allow_net: Vec<String>,
        /// Run the static type checker before execution (advisory — warnings only by default).
        #[arg(long)]
        type_check: bool,
        /// Treat type-check errors as hard errors (requires --type-check).
        #[arg(long)]
        strict_types: bool,
    },
    /// Inspect / disassemble a compiled bytecode file
    Inspect {
        /// Compiled bytecode file (.txtc)
        file: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Compile a Txt-code program to bytecode
    Compile {
        /// Input file
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Optimization level (none, basic, aggressive)
        #[arg(long, default_value = "basic")]
        optimize: String,
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
        /// Output results as JSON array
        #[arg(long)]
        json: bool,
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
        /// Target version (e.g., "0.4.0")
        #[arg(long)]
        to: Option<String>,
        /// Dry run: preview changes without modifying files
        #[arg(long)]
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
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Lint and type-check file(s) without executing
    Check {
        /// Input file(s)
        files: Vec<PathBuf>,
        /// Output issues as JSON array
        #[arg(long)]
        json: bool,
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
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
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
    if let Err(e) = txtcode::config::Config::ensure_directories() {
        eprintln!("Warning: Failed to initialize txtcode directories: {}", e);
    }
    if let Err(e) = txtcode::config::Config::init_default_config() {
        eprintln!("Warning: Failed to initialize config file: {}", e);
    }
    if let Err(e) = txtcode::tools::logger::init_logger("txtcode") {
        eprintln!("Warning: Failed to initialize logger: {}", e);
    }

    let cli = Cli::parse();

    let user_config = Config::load_config().unwrap_or_default();
    let safe_mode = cli.safe_mode || user_config.runtime.safe_mode;
    let allow_exec = cli.allow_exec || user_config.runtime.allow_exec;
    let debug = cli.debug || user_config.runtime.debug;
    let verbose = cli.verbose || user_config.runtime.verbose;
    let quiet = cli.quiet;

    if cli.command.is_none() && cli.file.is_none() && cli.verbose {
        doctor::print_verbose_info();
        return;
    }

    if let Some(code) = &cli.eval {
        if let Err(e) = eval_snippet(code, safe_mode, allow_exec, debug, verbose) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    match (&cli.command, &cli.file) {
        (None, None) => {
            if let Err(e) = repl_cli::start_repl(safe_mode, allow_exec, debug, verbose, quiet) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (None, Some(file)) => {
            if file.to_str() == Some("-") {
                use std::io::Read;
                let mut src = String::new();
                std::io::stdin().read_to_string(&mut src).unwrap_or(0);
                if let Err(e) = eval_snippet(&src, safe_mode, allow_exec, debug, verbose) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            if let Err(e) = run_cli::run_file(file, safe_mode, allow_exec, debug, verbose) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (Some(cmd), _) => {
            match cmd {
                Commands::Run {
                    file,
                    timeout,
                    sandbox,
                    env_file,
                    no_color,
                    json,
                    watch,
                    allow_fs,
                    allow_net,
                    type_check,
                    strict_types,
                } => {
                    if *no_color || std::env::var_os("NO_COLOR").is_some() {
                        std::env::set_var("NO_COLOR", "1");
                    }
                    if let Some(env_path) = env_file {
                        if let Err(e) = run_cli::load_env_file(env_path) {
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

                    // Optional static type check before execution.
                    // Only runs on .tc source files (bytecode files skip this step).
                    if *type_check && file.extension().and_then(|e| e.to_str()) == Some("tc") {
                        match std::fs::read_to_string(file) {
                            Ok(source) => {
                                let mut lexer = Lexer::new(source);
                                match lexer.tokenize() {
                                    Ok(tokens) => {
                                        let mut parser = Parser::new(tokens);
                                        match parser.parse() {
                                            Ok(program) => {
                                                let mut checker = TypeChecker::new();
                                                if let Err(type_errors) = checker.check(&program) {
                                                    for err in &type_errors {
                                                        if *strict_types {
                                                            eprintln!("type error: {}", err);
                                                        } else {
                                                            eprintln!("type warning: {}", err);
                                                        }
                                                    }
                                                    if *strict_types && !type_errors.is_empty() {
                                                        std::process::exit(1);
                                                    }
                                                }
                                            }
                                            Err(e) => eprintln!("type-check parse error: {}", e),
                                        }
                                    }
                                    Err(e) => eprintln!("type-check lex error: {}", e),
                                }
                            }
                            Err(e) => eprintln!("type-check read error: {}", e),
                        }
                    }

                    if *watch {
                        run_cli::run_file_watch(
                            file,
                            effective_safe,
                            effective_allow_exec,
                            debug,
                            verbose,
                            allow_fs.clone(),
                            allow_net.clone(),
                        );
                        return;
                    }
                    let result = if let Some(ts) = timeout {
                        run_cli::run_file_with_timeout(
                            file,
                            effective_safe,
                            effective_allow_exec,
                            debug,
                            verbose,
                            ts,
                            allow_fs,
                            allow_net,
                        )
                    } else {
                        run_cli::run_file_with_allowlists(
                            file,
                            effective_safe,
                            effective_allow_exec,
                            debug,
                            verbose,
                            allow_fs,
                            allow_net,
                        )
                    };
                    if let Err(e) = result {
                        if *json {
                            let msg = e.to_string().replace('"', "\\\"");
                            let code = txtcode::runtime::errors::ErrorCode::infer_from_message(&msg);
                            eprintln!(
                                "{{\"error\":\"{}\",\"type\":\"RuntimeError\",\"code\":\"{}\"}}",
                                msg, code.as_str()
                            );
                        } else {
                            eprintln!("Error: {}", e);
                        }
                        std::process::exit(1);
                    }
                }
                Commands::Compile {
                    file,
                    output,
                    optimize,
                } => {
                    if let Err(e) = compile::compile_file(file, output.as_ref(), optimize) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Inspect { file, format } => {
                    if let Err(e) = compile::inspect_bytecode(file, format) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Format {
                    files,
                    write,
                    check,
                    json,
                } => {
                    if *write && *check {
                        eprintln!("Error: --write and --check are mutually exclusive.");
                        std::process::exit(1);
                    }
                    if let Err(e) = format_cli::format_files(files, *write, *check, *json) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Lint { files, format, fix } => {
                    if let Err(e) = lint_cli::lint_files(files, format, *fix) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Repl => {
                    if let Err(e) =
                        repl_cli::start_repl(safe_mode, allow_exec, debug, verbose, quiet)
                    {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Package { command } => {
                    if let Err(e) = handle_package_command(command) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Debug { file } => {
                    if let Err(e) = debug_cli::start_debug_repl(file) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Migrate {
                    files,
                    directory,
                    from,
                    to,
                    dry_run,
                    strict,
                } => {
                    use txtcode::cli::migrate::migrate_command;
                    if let Err(e) = migrate_command(
                        files.clone(),
                        from.clone(),
                        to.clone(),
                        *dry_run,
                        *strict,
                        directory.clone(),
                    ) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Init { name } => {
                    if let Err(e) = init::init_project(name.as_deref()) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Doctor => {
                    doctor::run_doctor();
                }
                Commands::Test {
                    path,
                    filter,
                    watch,
                    json,
                } => {
                    if *watch {
                        if let Err(e) = test_cmd::run_tests_watch(path, filter.as_deref()) {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    } else if let Err(e) = test_cmd::run_tests(path, filter.as_deref(), *json) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Check { files, json } => {
                    if let Err(e) = check::check_files(files, *json) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Doc {
                    files,
                    output,
                    format,
                } => {
                    if let Err(e) = doc::generate_docs(files, output.as_ref(), format) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                Commands::Bench {
                    file,
                    runs,
                    warmup,
                    save,
                    compare,
                } => {
                    if let Err(e) =
                        bench::benchmark_file(file, *runs, *warmup, save.as_ref(), compare.as_ref())
                    {
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

// ── Enum dispatchers (stay in binary — enums are defined here) ─────────────────

fn handle_env_command(command: &EnvCommands) -> Result<(), String> {
    match command {
        EnvCommands::Init { name, sandbox, all } => {
            env_cli::env_init(name.clone(), *sandbox, *all).map_err(|e| e.to_string())
        }
        EnvCommands::Install => env_cli::env_install().map_err(|e| e.to_string()),
        EnvCommands::Use { name } => env_cli::env_use(name).map_err(|e| e.to_string()),
        EnvCommands::Status { json } => {
            if *json {
                env_cli::env_status_json().map_err(|e| e.to_string())
            } else {
                env_cli::env_status().map_err(|e| e.to_string())
            }
        }
        EnvCommands::List => env_cli::env_list().map_err(|e| e.to_string()),
        EnvCommands::Clean { name } => env_cli::env_clean(name.clone()).map_err(|e| e.to_string()),
        EnvCommands::Remove { name } => {
            env_cli::env_remove(name.clone()).map_err(|e| e.to_string())
        }
        EnvCommands::Doctor => env_cli::env_doctor().map_err(|e| e.to_string()),
        EnvCommands::Diff { a, b } => env_cli::env_diff(a, b).map_err(|e| e.to_string()),
        EnvCommands::Freeze => env_cli::env_freeze().map_err(|e| e.to_string()),
        EnvCommands::ShellHook => env_cli::env_shell_hook().map_err(|e| e.to_string()),
        EnvCommands::Path => env_cli::env_path().map_err(|e| e.to_string()),
    }
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

// ── Inline eval (belongs at binary entry point) ────────────────────────────────

fn eval_snippet(
    code: &str,
    safe_mode: bool,
    allow_exec: bool,
    debug: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut vm = VirtualMachine::with_all_options(safe_mode, debug, verbose);
    vm.set_exec_allowed(allow_exec);
    let mut lexer = Lexer::new(code.to_string());
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    Validator::validate_program(&program)
        .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
    let result = vm.interpret(&program)?;
    if !matches!(result, Value::Null) {
        println!("{}", Value::to_string(&result));
    }
    Ok(())
}

// ── Package registry lookup (no standalone module — package.rs is stable) ────

fn package_search(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Searching packages for '{}'...\n", query);

    let registry_url = std::env::var("TXTCODE_REGISTRY").ok();

    if let Some(url) = registry_url {
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
    let toml_path = PathBuf::from("Txtcode.toml");
    if toml_path.exists() {
        let content = std::fs::read_to_string(&toml_path)?;
        let in_local = content.lines().any(|l| {
            let t = l.trim();
            t.starts_with(name) && t.contains('=')
        });
        if in_local {
            println!("Package: {}", name);
            println!("Source:  local Txtcode.toml");
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with(name) && t.contains('=') {
                    println!("Entry:   {}", t);
                }
            }
            return Ok(());
        }
    }

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
