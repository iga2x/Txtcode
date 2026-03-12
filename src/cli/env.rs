/// txtcode env — Virtual environment management
///
/// Provides project-local package isolation, named permission profiles
/// (dev / prod / test / sandbox), and auto-detection when running scripts.
///
/// Directory layout created in the user's project:
/// ```text
/// .txtcode-env/
/// +-- active               (name of the current env, e.g. "dev")
/// +-- dev/
/// |   +-- env.toml         (permissions + settings for "dev")
/// |   +-- packages/        (locally installed packages for "dev")
/// +-- prod/
/// |   +-- env.toml
/// |   +-- packages/
/// +-- sandbox/
///     +-- env.toml
///     +-- packages/
/// ```
use crate::config::{Config, EnvConfig, DEFAULT_ENV_NAME, LOCAL_ENV_DIR};
use crate::runtime::errors::RuntimeError;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// env.toml TEMPLATES
// ─────────────────────────────────────────────────────────────────────────────

const TEMPLATE_DEV: &str = r#"[env]
name        = "dev"
version     = "0.4.0"
description = "Development environment — full access, verbose output"

[permissions]
allow     = ["fs.read", "fs.write", "net.connect", "sys.exec", "sys.env"]
deny      = []
safe_mode = false

[settings]
timeout          = "none"
max_memory       = "1gb"
verbose          = true
use_local_packages = true
"#;

const TEMPLATE_PROD: &str = r#"[env]
name        = "prod"
version     = "0.4.0"
description = "Production environment — restricted permissions, safe mode on"

[permissions]
allow     = ["fs.read:/app/*", "net.connect", "sys.env"]
deny      = ["sys.exec", "fs.write:/etc/*", "fs.delete"]
safe_mode = true

[settings]
timeout          = "30s"
max_memory       = "256mb"
verbose          = false
use_local_packages = true
"#;

const TEMPLATE_TEST: &str = r#"[env]
name        = "test"
version     = "0.4.0"
description = "Test environment — full fs/net, no process spawning"

[permissions]
allow     = ["fs.read", "fs.write:/tmp/*", "net.connect", "sys.env"]
deny      = ["sys.exec"]
safe_mode = false

[settings]
timeout          = "60s"
max_memory       = "512mb"
verbose          = true
use_local_packages = true
"#;

const TEMPLATE_SANDBOX: &str = r#"[env]
name        = "sandbox"
version     = "0.4.0"
description = "Sandbox environment — zero trust, nothing allowed by default"

[permissions]
allow     = []
deny      = ["fs.read", "fs.write", "fs.delete", "net.connect", "sys.exec", "sys.env"]
safe_mode = true

[settings]
timeout          = "10s"
max_memory       = "64mb"
verbose          = false
use_local_packages = true
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the `.txtcode-env/` directory from current working directory.
/// Returns an error if no env exists and `require` is true.
fn require_env_dir(require: bool) -> Result<Option<PathBuf>, RuntimeError> {
    let cwd = std::env::current_dir()
        .map_err(|e| RuntimeError::new(format!("Cannot read cwd: {}", e)))?;
    match Config::detect_local_env(&cwd) {
        Some(p) => Ok(Some(p)),
        None if require => Err(RuntimeError::new(
            "No .txtcode-env/ found in this directory or any parent.\n\
             Run `txtcode env init` first to create one."
                .to_string(),
        )),
        None => Ok(None),
    }
}

/// Create a directory (and all parents), with a helpful error on failure.
fn mkdir(path: &Path) -> Result<(), RuntimeError> {
    fs::create_dir_all(path)
        .map_err(|e| RuntimeError::new(format!("Failed to create {}: {}", path.display(), e)))
}

/// Write a file, creating parent dirs first.
fn write_file(path: &Path, content: &str) -> Result<(), RuntimeError> {
    if let Some(parent) = path.parent() {
        mkdir(parent)?;
    }
    fs::write(path, content)
        .map_err(|e| RuntimeError::new(format!("Failed to write {}: {}", path.display(), e)))
}

/// Return the template string for a given preset name.
fn template_for(name: &str, sandbox: bool) -> &'static str {
    if sandbox || name == "sandbox" {
        TEMPLATE_SANDBOX
    } else {
        match name {
            "prod" | "production" => TEMPLATE_PROD,
            "test" | "testing" => TEMPLATE_TEST,
            _ => TEMPLATE_DEV, // dev / default / custom
        }
    }
}

/// Print a status line. ok=true → green tick, ok=false → red cross.
fn status_line(ok: bool, label: &str, detail: &str) {
    let icon = if ok { "✅".green() } else { "❌".red() };
    println!("  {} {} {}", icon, label.bold(), detail.dimmed());
}

fn warn_line(label: &str, detail: &str) {
    println!("  {} {} {}", "⚠️ ".yellow(), label.bold(), detail.dimmed());
}

// ─────────────────────────────────────────────────────────────────────────────
// Public commands
// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env init [--name <name>] [--sandbox] [--all]`
///
/// Creates `.txtcode-env/` in the current directory with the requested
/// named profile (default: "dev").  `--all` creates dev + prod + test + sandbox.
pub fn env_init(
    name: Option<String>,
    sandbox: bool,
    all_presets: bool,
) -> Result<(), RuntimeError> {
    let cwd = std::env::current_dir()
        .map_err(|e| RuntimeError::new(format!("Cannot read cwd: {}", e)))?;

    let env_dir = cwd.join(LOCAL_ENV_DIR);

    if all_presets {
        // Create all four standard presets
        for preset in &["dev", "prod", "test", "sandbox"] {
            create_single_env(&env_dir, preset, preset == &"sandbox")?;
        }
        // Set dev as the default active env
        Config::set_active_env_name(&env_dir, "dev").map_err(RuntimeError::new)?;
        println!(
            "{}",
            "✅ Created all preset environments: dev, prod, test, sandbox"
                .green()
                .bold()
        );
        println!("   Active env set to: {}", "dev".cyan().bold());
    } else {
        let env_name = name.as_deref().unwrap_or(DEFAULT_ENV_NAME);
        create_single_env(&env_dir, env_name, sandbox)?;
        Config::set_active_env_name(&env_dir, env_name).map_err(RuntimeError::new)?;
        println!(
            "{} {}",
            "✅ Created environment:".green().bold(),
            env_name.cyan().bold()
        );
        println!("   Location: {}", env_dir.join(env_name).display());
    }

    println!(
        "\n   Run {} to install dependencies.",
        "txtcode env install".cyan()
    );
    Ok(())
}

/// Create a single named env inside `env_dir`.
fn create_single_env(env_dir: &Path, name: &str, sandbox: bool) -> Result<(), RuntimeError> {
    let named_dir = env_dir.join(name);
    let pkg_dir = named_dir.join("packages");
    let cache_dir = named_dir.join("cache");
    let env_toml = named_dir.join("env.toml");

    mkdir(&pkg_dir)?;
    mkdir(&cache_dir)?;

    // Only write env.toml if it doesn't already exist (don't overwrite)
    if !env_toml.exists() {
        write_file(&env_toml, template_for(name, sandbox))?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env install`
///
/// Reads `Txtcode.toml` in cwd, resolves dependencies, and installs them
/// into `.txtcode-env/{active}/packages/`.
pub fn env_install() -> Result<(), RuntimeError> {
    let env_dir = require_env_dir(true)?.unwrap();
    let name = Config::get_active_env_name(&env_dir);
    let pkg_dir = env_dir.join(&name).join("packages");

    println!(
        "{} {} {}",
        "📦 Installing dependencies for env".bold(),
        name.cyan().bold(),
        format!("→ {}", pkg_dir.display()).dimmed(),
    );

    mkdir(&pkg_dir)?;

    // Load Txtcode.toml
    let manifest_path = std::env::current_dir()
        .map_err(|e| RuntimeError::new(e.to_string()))?
        .join("Txtcode.toml");

    if !manifest_path.exists() {
        return Err(RuntimeError::new(
            "No Txtcode.toml found. Run `txtcode init` to create a project manifest.".to_string(),
        ));
    }

    // Delegate to PackageRegistry with the local packages dir
    use crate::cli::package::{PackageConfig, PackageRegistry};
    let config = PackageConfig::load(&manifest_path)
        .map_err(|e| RuntimeError::new(format!("Failed to load Txtcode.toml: {}", e)))?;

    if config.dependencies.is_empty() {
        println!("  No dependencies declared in Txtcode.toml.");
        return Ok(());
    }

    let registry = PackageRegistry::new(pkg_dir.clone());

    let mut installed = 0usize;
    for (dep_name, version_req) in &config.dependencies {
        print!("  {} {} @ {} ... ", "→".cyan(), dep_name, version_req);
        match registry.download_package(dep_name, version_req) {
            Ok(_) => {
                println!("{}", "ok".green());
                installed += 1;
            }
            Err(e) => {
                println!("{} ({})", "failed".red(), e);
            }
        }
    }

    println!(
        "\n  {}/{} packages installed into env {}.",
        installed,
        config.dependencies.len(),
        name.cyan().bold(),
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env use <name>`
///
/// Switch the active environment for this project.
pub fn env_use(name: &str) -> Result<(), RuntimeError> {
    let env_dir = require_env_dir(true)?.unwrap();
    let named_dir = env_dir.join(name);

    if !named_dir.is_dir() {
        return Err(RuntimeError::new(format!(
            "Environment '{}' does not exist.\n\
             Available: {}\n\
             Create it with: txtcode env init --name {}",
            name,
            list_env_names(&env_dir).join(", "),
            name,
        )));
    }

    Config::set_active_env_name(&env_dir, name).map_err(RuntimeError::new)?;

    println!("{} Active env set to: {}", "✅".green(), name.cyan().bold());

    // Show permission profile
    if let Ok(cfg) = Config::load_env_config(&env_dir, name) {
        print_permission_summary(&cfg);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env status`
///
/// Show the currently active env, its permission profile, and package count.
pub fn env_status() -> Result<(), RuntimeError> {
    match Config::load_active_env() {
        None => {
            println!(
                "{}",
                "No .txtcode-env/ found in this directory tree.".yellow()
            );
            println!("Run {} to create one.", "txtcode env init".cyan());
        }
        Some((env_dir, name, config)) => {
            println!(
                "{}",
                "── Txt-code Environment Status ──────────────────".bold()
            );
            println!("  Active env : {}", name.cyan().bold());
            println!("  Location   : {}", env_dir.display());
            println!("  Version    : {}", config.env.version);
            if !config.env.description.is_empty() {
                println!("  Description: {}", config.env.description);
            }

            println!();
            print_permission_summary(&config);

            // Package count
            let pkg_dir = env_dir.join(&name).join("packages");
            let pkg_count = count_packages(&pkg_dir);
            println!();
            println!(
                "  Packages: {} installed in {}",
                pkg_count.to_string().cyan(),
                pkg_dir.display(),
            );

            // Timeout / memory
            println!("  Timeout  : {}", config.settings.timeout);
            println!("  Max mem  : {}", config.settings.max_memory);
        }
    }
    Ok(())
}

/// `txtcode env status --json`
pub fn env_status_json() -> Result<(), RuntimeError> {
    match Config::load_active_env() {
        None => {
            println!("{{\"active\":null,\"error\":\"No .txtcode-env/ found\"}}");
        }
        Some((env_dir, name, config)) => {
            let pkg_dir = env_dir.join(&name).join("packages");
            let pkg_count = count_packages(&pkg_dir);
            let allow_json = config
                .permissions
                .allow
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(",");
            let deny_json = config
                .permissions
                .deny
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "{{\"active\":\"{}\",\"location\":\"{}\",\"version\":\"{}\",\
                 \"description\":\"{}\",\"safe_mode\":{},\"allow\":[{}],\
                 \"deny\":[{}],\"packages\":{},\"timeout\":\"{}\",\"max_memory\":\"{}\"}}",
                name,
                env_dir.display(),
                config.env.version,
                config.env.description.replace('"', "\\\""),
                config.permissions.safe_mode,
                allow_json,
                deny_json,
                pkg_count,
                config.settings.timeout,
                config.settings.max_memory,
            );
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env list`
///
/// List all named environments in the current project.
pub fn env_list() -> Result<(), RuntimeError> {
    let env_dir = match require_env_dir(false)? {
        None => {
            println!("{}", "No .txtcode-env/ found.".yellow());
            return Ok(());
        }
        Some(d) => d,
    };

    let active = Config::get_active_env_name(&env_dir);
    let names = list_env_names(&env_dir);

    if names.is_empty() {
        println!("No environments found in {}.", env_dir.display());
        println!("Run {} to create one.", "txtcode env init".cyan());
        return Ok(());
    }

    println!(
        "{}",
        "── Environments ─────────────────────────────────────".bold()
    );
    for name in &names {
        let marker = if *name == active {
            " ◀ active".green().to_string()
        } else {
            String::new()
        };
        let safe_tag = if let Ok(cfg) = Config::load_env_config(&env_dir, name) {
            if cfg.permissions.safe_mode {
                " [safe]".yellow().to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let pkg_count = count_packages(&env_dir.join(name).join("packages"));
        println!(
            "  {:12}  {} packages{}{}",
            name.cyan().bold(),
            pkg_count,
            safe_tag,
            marker
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env clean [--name <name>]`
///
/// Remove installed packages from the env but keep env.toml.
pub fn env_clean(name: Option<String>) -> Result<(), RuntimeError> {
    let env_dir = require_env_dir(true)?.unwrap();
    let env_name = name.unwrap_or_else(|| Config::get_active_env_name(&env_dir));
    let pkg_dir = env_dir.join(&env_name).join("packages");

    if !pkg_dir.exists() {
        println!("Nothing to clean (packages dir does not exist).");
        return Ok(());
    }

    fs::remove_dir_all(&pkg_dir)
        .map_err(|e| RuntimeError::new(format!("Failed to remove packages: {}", e)))?;
    mkdir(&pkg_dir)?;

    println!(
        "{} Cleaned packages for env {}.",
        "✅".green(),
        env_name.cyan().bold()
    );
    println!("   Run {} to reinstall.", "txtcode env install".cyan());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env remove [--name <name>]`
///
/// Completely remove a named env (env.toml + packages).
/// If removing the active env, resets active to "dev".
pub fn env_remove(name: Option<String>) -> Result<(), RuntimeError> {
    let env_dir = require_env_dir(true)?.unwrap();
    let env_name = name.unwrap_or_else(|| Config::get_active_env_name(&env_dir));
    let named_dir = env_dir.join(&env_name);

    if !named_dir.exists() {
        return Err(RuntimeError::new(format!(
            "Environment '{}' does not exist.",
            env_name
        )));
    }

    fs::remove_dir_all(&named_dir)
        .map_err(|e| RuntimeError::new(format!("Failed to remove env: {}", e)))?;

    // If we removed the active env, reset to dev (or first remaining)
    let active = Config::get_active_env_name(&env_dir);
    if active == env_name {
        let remaining = list_env_names(&env_dir);
        let fallback = remaining.first().map(|s| s.as_str()).unwrap_or("dev");
        let _ = Config::set_active_env_name(&env_dir, fallback);
    }

    println!(
        "{} Removed environment {}.",
        "✅".green(),
        env_name.cyan().bold()
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env doctor`
///
/// Validate the local environment: check env.toml, lockfile consistency,
/// package integrity, and flag any security concerns in the permission profile.
pub fn env_doctor() -> Result<(), RuntimeError> {
    println!(
        "{}",
        "── Txt-code Env Doctor ──────────────────────────────".bold()
    );

    let cwd = std::env::current_dir().map_err(|e| RuntimeError::new(e.to_string()))?;

    // 1. Local env directory
    let env_dir = match Config::detect_local_env(&cwd) {
        Some(d) => {
            status_line(true, ".txtcode-env/", &d.display().to_string());
            d
        }
        None => {
            status_line(false, ".txtcode-env/", "not found — run `txtcode env init`");
            return Ok(());
        }
    };

    // 2. Active env file
    let active = Config::get_active_env_name(&env_dir);
    status_line(true, "Active env:", &active);

    let named_dir = env_dir.join(&active);
    status_line(
        named_dir.is_dir(),
        &format!(".txtcode-env/{}/", active),
        "exists",
    );

    // 3. env.toml
    let env_toml = named_dir.join("env.toml");
    if env_toml.exists() {
        status_line(true, "env.toml", "found");
        match Config::load_env_config(&env_dir, &active) {
            Ok(cfg) => {
                // Security checks
                if !cfg.permissions.safe_mode {
                    warn_line("safe_mode", "disabled — scripts can spawn processes");
                }
                let has_exec = cfg.permissions.allow.iter().any(|a| a.contains("sys.exec"));
                if has_exec && active == "prod" {
                    warn_line("sys.exec", "allowed in prod env — consider denying it");
                }
                let has_wildcard_fs_write = cfg
                    .permissions
                    .allow
                    .iter()
                    .any(|a| a == "fs.write" || a == "fs.*");
                if has_wildcard_fs_write {
                    warn_line(
                        "fs.write",
                        "unrestricted write — consider scoping to a path",
                    );
                }
                status_line(
                    true,
                    "Permissions",
                    &format!(
                        "{} allow / {} deny",
                        cfg.permissions.allow.len(),
                        cfg.permissions.deny.len()
                    ),
                );
            }
            Err(e) => status_line(false, "env.toml parse", &e),
        }
    } else {
        status_line(
            false,
            "env.toml",
            "missing — run `txtcode env init --name {env}`",
        );
    }

    // 4. Packages directory
    let pkg_dir = named_dir.join("packages");
    status_line(
        pkg_dir.exists(),
        "packages/",
        &format!("{} installed", count_packages(&pkg_dir)),
    );

    // 5. Txtcode.toml
    let manifest = cwd.join("Txtcode.toml");
    status_line(
        manifest.exists(),
        "Txtcode.toml",
        if manifest.exists() {
            "found"
        } else {
            "missing"
        },
    );

    // 6. Txtcode.lock
    let lock = cwd.join("Txtcode.lock");
    status_line(
        lock.exists(),
        "Txtcode.lock",
        if lock.exists() {
            "found"
        } else {
            "not yet generated"
        },
    );

    println!("\n{}", "Doctor complete.".bold());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env diff <name-a> <name-b>`
///
/// Compare permission profiles between two named environments.
pub fn env_diff(name_a: &str, name_b: &str) -> Result<(), RuntimeError> {
    let env_dir = require_env_dir(true)?.unwrap();

    let cfg_a = Config::load_env_config(&env_dir, name_a).map_err(RuntimeError::new)?;
    let cfg_b = Config::load_env_config(&env_dir, name_b).map_err(RuntimeError::new)?;

    println!(
        "{}",
        format!(
            "── Diff: {} vs {} ─────────────────────────────",
            name_a, name_b
        )
        .bold()
    );

    diff_vec(
        "allow",
        &cfg_a.permissions.allow,
        &cfg_b.permissions.allow,
        name_a,
        name_b,
    );
    diff_vec(
        "deny",
        &cfg_a.permissions.deny,
        &cfg_b.permissions.deny,
        name_a,
        name_b,
    );
    diff_bool(
        "safe_mode",
        cfg_a.permissions.safe_mode,
        cfg_b.permissions.safe_mode,
        name_a,
        name_b,
    );

    println!();
    diff_str(
        "timeout",
        &cfg_a.settings.timeout,
        &cfg_b.settings.timeout,
        name_a,
        name_b,
    );
    diff_str(
        "max_memory",
        &cfg_a.settings.max_memory,
        &cfg_b.settings.max_memory,
        name_a,
        name_b,
    );
    Ok(())
}

fn diff_vec(key: &str, a: &[String], b: &[String], na: &str, nb: &str) {
    let only_a: Vec<_> = a.iter().filter(|x| !b.contains(x)).collect();
    let only_b: Vec<_> = b.iter().filter(|x| !a.contains(x)).collect();
    for v in &only_a {
        println!("  {} {} = {} (only in {})", "-".red(), key, v.red(), na);
    }
    for v in &only_b {
        println!("  {} {} = {} (only in {})", "+".green(), key, v.green(), nb);
    }
}

fn diff_bool(key: &str, a: bool, b: bool, na: &str, nb: &str) {
    if a != b {
        println!(
            "  {} {} = {} ({}) vs {} ({})",
            "~".yellow(),
            key,
            a,
            na,
            b,
            nb
        );
    }
}

fn diff_str(key: &str, a: &str, b: &str, na: &str, nb: &str) {
    if a != b {
        println!(
            "  {} {} = {} ({}) vs {} ({})",
            "~".yellow(),
            key,
            a,
            na,
            b,
            nb
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env freeze`
///
/// Print the current env config to stdout (pipe to a file to save a snapshot).
pub fn env_freeze() -> Result<(), RuntimeError> {
    let env_dir = require_env_dir(true)?.unwrap();
    let name = Config::get_active_env_name(&env_dir);
    let cfg = Config::load_env_config(&env_dir, &name).map_err(RuntimeError::new)?;

    let out = toml::to_string_pretty(&cfg)
        .map_err(|e| RuntimeError::new(format!("Serialize error: {}", e)))?;

    println!(
        "# Snapshot of env '{}' — {}",
        name,
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
    println!("{}", out);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// `txtcode env shell-hook`
///
/// Outputs a shell function that can be eval'd to show the active env in
/// the user's prompt.  Usage: eval "$(txtcode env shell-hook)"
pub fn env_shell_hook() -> Result<(), RuntimeError> {
    // Detect shell
    let shell = std::env::var("SHELL").unwrap_or_default();
    let is_fish = shell.contains("fish");

    if is_fish {
        println!(
            r#"
# Txt-code env shell hook (fish)
function __txtcode_env_prompt
    set -l env_dir (txtcode env path 2>/dev/null)
    if test -n "$env_dir"
        set -l active (cat "$env_dir/active" 2>/dev/null)
        if test -n "$active"
            echo -n "[$active] "
        end
    end
end
"#
        );
    } else {
        // bash / zsh
        println!(
            r#"
# Txt-code env shell hook (bash/zsh)
# Add to ~/.bashrc or ~/.zshrc:
#   eval "$(txtcode env shell-hook)"
__txtcode_env_ps1() {{
    local env_dir
    env_dir=$(txtcode env path 2>/dev/null)
    if [ -n "$env_dir" ] && [ -f "$env_dir/active" ]; then
        local active
        active=$(cat "$env_dir/active")
        [ -n "$active" ] && echo -n "[$active] "
    fi
}}
export PS1='$(__txtcode_env_ps1)'"$PS1"
"#
        );
    }
    Ok(())
}

/// `txtcode env path`
///
/// Print the path to the active .txtcode-env/ directory (used by shell hook).
pub fn env_path() -> Result<(), RuntimeError> {
    match require_env_dir(false)? {
        Some(p) => {
            println!("{}", p.display());
            Ok(())
        }
        None => Err(RuntimeError::new("No .txtcode-env/ found.".to_string())),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// List all named env directories inside `env_dir` (excludes the `active` file).
fn list_env_names(env_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(env_dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Count the number of package directories inside `pkg_dir`.
fn count_packages(pkg_dir: &Path) -> usize {
    if !pkg_dir.exists() {
        return 0;
    }
    fs::read_dir(pkg_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0)
}

/// Print a compact permission summary to stdout.
fn print_permission_summary(config: &EnvConfig) {
    let safe = if config.permissions.safe_mode {
        " [safe-mode ON]".yellow().to_string()
    } else {
        String::new()
    };
    println!("  Permissions:{}", safe);
    if config.permissions.allow.is_empty() {
        println!("    allow: {} (nothing allowed)", "∅".red());
    } else {
        for cap in &config.permissions.allow {
            println!("    {} {}", "allow".green(), cap);
        }
    }
    for cap in &config.permissions.deny {
        println!("    {} {}", "deny ".red(), cap);
    }
}
