use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// txtcode self uninstall
// ─────────────────────────────────────────────────────────────────────────────

pub fn self_uninstall(yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════╗");
    println!("║     Txt-code Uninstall                   ║");
    println!("╚══════════════════════════════════════════╝\n");

    // Locate the running binary
    let bin_path = std::env::current_exe()
        .map_err(|e| format!("Cannot locate txtcode binary: {}", e))?;

    // Locate global data directory
    let global_data = dirs::home_dir()
        .map(|h| h.join(".txtcode"))
        .unwrap_or_else(|| PathBuf::from(".txtcode"));

    // Show the three modes
    println!("What would you like to remove?\n");
    println!("  1) Binary only            (safest)");
    println!("     Removes : {}", bin_path.display());
    println!("     Keeps   : ~/.txtcode/  and all project .txtcode-env/ dirs\n");
    println!("  2) Binary + global data");
    println!("     Removes : {}", bin_path.display());
    println!("     Removes : {}  (config, cache, logs, global packages)", global_data.display());
    println!("     Keeps   : All project .txtcode-env/ dirs untouched\n");
    println!("  3) Complete wipe          (everything)");
    println!("     Removes : {}", bin_path.display());
    println!("     Removes : {}", global_data.display());
    println!("     Removes : All .txtcode-env/ dirs found under your home directory");
    println!("     WARNING : This cannot be undone\n");

    let choice = if yes {
        println!("(--yes flag set, defaulting to mode 1: binary only)");
        1u8
    } else {
        print!("Enter choice [1/2/3] (or q to quit): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed == "q" || trimmed == "Q" {
            println!("Uninstall cancelled.");
            return Ok(());
        }
        match trimmed.parse::<u8>() {
            Ok(n) if n >= 1 && n <= 3 => n,
            _ => {
                eprintln!("Invalid choice. Uninstall cancelled.");
                return Ok(());
            }
        }
    };

    // Collect what will be deleted
    let mut to_delete: Vec<PathBuf> = Vec::new();
    let mut project_envs: Vec<PathBuf> = Vec::new();

    to_delete.push(bin_path.clone());

    if choice >= 2 && global_data.exists() {
        to_delete.push(global_data.clone());
    }

    if choice == 3 {
        project_envs = find_project_envs();
        to_delete.extend(project_envs.clone());
    }

    // Show what will be deleted
    println!("\nThe following will be permanently deleted:");
    for path in &to_delete {
        println!("  - {}", path.display());
    }
    if project_envs.is_empty() && choice == 3 {
        println!("  (No .txtcode-env/ directories found under home)");
    }

    // Confirm (skip if --yes and mode 1 or 2; always confirm mode 3)
    let confirmed = if choice == 3 {
        // Mode 3 always asks, even with --yes
        print!("\nType 'DELETE ALL' to confirm complete wipe: ");
        io::stdout().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;
        confirm.trim() == "DELETE ALL"
    } else if yes {
        true
    } else {
        print!("\nProceed? [y/N]: ");
        io::stdout().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;
        confirm.trim().eq_ignore_ascii_case("y")
    };

    if !confirmed {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    // Execute deletions
    println!("\nUninstalling...");

    // Remove project envs first (before binary, so errors don't leave us without the binary)
    for env_dir in &project_envs {
        match fs::remove_dir_all(env_dir) {
            Ok(_) => println!("  ✓ Removed {}", env_dir.display()),
            Err(e) => eprintln!("  ✗ Failed to remove {}: {}", env_dir.display(), e),
        }
    }

    // Remove global data
    if choice >= 2 && global_data.exists() {
        match fs::remove_dir_all(&global_data) {
            Ok(_) => println!("  ✓ Removed {}", global_data.display()),
            Err(e) => eprintln!("  ✗ Failed to remove {}: {}", global_data.display(), e),
        }
    }

    // Remove PATH entries from shell config files
    clean_path_entries();

    // Remove binary last (after it, this process ends)
    println!("  ✓ Removing binary: {}", bin_path.display());
    // We schedule binary removal after the process exits using a temp shell script
    remove_binary_deferred(&bin_path)?;

    println!("\nTxt-code has been uninstalled.");
    if choice == 1 {
        println!("Your project files and ~/.txtcode/ data are untouched.");
    } else if choice == 2 {
        println!("Your project .txtcode-env/ directories are untouched.");
    } else {
        println!("All Txt-code data has been removed.");
    }
    println!("\nTo reinstall: curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh\n");

    Ok(())
}

/// Walk the user's home directory looking for .txtcode-env/ directories.
fn find_project_envs() -> Vec<PathBuf> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let mut found = Vec::new();
    walk_for_envs(&home, &mut found, 0);
    found
}

fn walk_for_envs(dir: &std::path::Path, found: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 { return; } // don't go too deep
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == ".txtcode-env" {
                found.push(path);
            } else if !name.starts_with('.') {
                // recurse into non-hidden dirs
                walk_for_envs(&path, found, depth + 1);
            }
        }
    }
}

/// Remove Txt-code PATH entries from common shell config files.
fn clean_path_entries() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let shell_files = [
        ".bashrc", ".bash_profile", ".zshrc", ".zprofile", ".profile",
    ];

    let marker = "# txtcode";

    for filename in &shell_files {
        let path = home.join(filename);
        if !path.exists() { continue; }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Filter out lines that contain the txtcode PATH addition
        let cleaned: Vec<&str> = content
            .lines()
            .filter(|line| {
                !line.contains(marker) && !line.contains("txtcode") ||
                // keep lines that just mention txtcode in comments not related to PATH
                (line.contains("txtcode") && !line.contains("PATH") && !line.contains("export"))
            })
            .collect();

        if cleaned.len() < content.lines().count() {
            let new_content = cleaned.join("\n") + "\n";
            if fs::write(&path, new_content).is_ok() {
                println!("  ✓ Cleaned PATH entry from ~/{}", filename);
            }
        }
    }
}

/// On Unix: write a tiny shell script that sleeps 1s then removes the binary,
/// then exec it in the background and exit this process.
/// On Windows: use a scheduled rename trick.
fn remove_binary_deferred(bin_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        let tmp = std::env::temp_dir().join("txtcode_remove.sh");
        let script = format!(
            "#!/bin/sh\nsleep 1\nrm -f \"{}\"\nrm -f \"$0\"\n",
            bin_path.display()
        );
        fs::write(&tmp, script)?;
        // Make executable
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        // Run in background and exit
        std::process::Command::new("sh")
            .arg(&tmp)
            .spawn()?;
    }
    #[cfg(not(unix))]
    {
        // On Windows, rename binary so it's unlocked, then remove
        let tmp_path = bin_path.with_extension("old");
        let _ = fs::rename(bin_path, &tmp_path);
        // Schedule deletion - best effort
        std::process::Command::new("cmd")
            .args(["/C", "ping", "-n", "2", "127.0.0.1", ">nul", "&",
                   "del", &tmp_path.to_string_lossy()])
            .spawn()?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// txtcode self update
// ─────────────────────────────────────────────────────────────────────────────

pub fn self_update() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════╗");
    println!("║     Txt-code Self-Update                 ║");
    println!("╚══════════════════════════════════════════╝\n");

    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version : v{}", current_version);
    println!("Checking for updates...\n");

    // Fetch latest version from GitHub releases API
    let latest = fetch_latest_version();

    match latest {
        Ok(latest_version) => {
            println!("Latest version  : v{}", latest_version);
            if latest_version == current_version {
                println!("\nYou are already on the latest version.");
                return Ok(());
            }
            println!("\nNew version available: v{} → v{}", current_version, latest_version);
            println!("\nTo update, run:");
            println!("  curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh");
            println!("\nOr if you installed from source:");
            println!("  cd /path/to/txtcode && git pull && cargo install --path .");
        }
        Err(_) => {
            println!("Could not reach GitHub to check for updates.");
            println!("Check your internet connection or visit:");
            println!("  https://github.com/iga2x/txtcode/releases");
        }
    }

    Ok(())
}

fn fetch_latest_version() -> Result<String, Box<dyn std::error::Error>> {
    // Use reqwest blocking to hit GitHub API
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("txtcode/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let resp = client
            .get("https://api.github.com/repos/iga2x/txtcode/releases/latest")
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let tag = json["tag_name"]
            .as_str()
            .unwrap_or("")
            .trim_start_matches('v')
            .to_string();

        if tag.is_empty() {
            Err("No release tag found".into())
        } else {
            Ok(tag)
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// txtcode self info
// ─────────────────────────────────────────────────────────────────────────────

pub fn self_info() -> Result<(), Box<dyn std::error::Error>> {
    let bin_path = std::env::current_exe().ok();
    let global_data = dirs::home_dir().map(|h| h.join(".txtcode"));

    println!("\nTxt-code Installation Info");
    println!("──────────────────────────");
    println!("Version      : v{}", env!("CARGO_PKG_VERSION"));

    if let Some(ref p) = bin_path {
        println!("Binary       : {}", p.display());
    }
    if let Some(ref d) = global_data {
        println!("Global data  : {}", d.display());
        if d.exists() {
            let size = dir_size(d);
            println!("Data size    : {}", human_size(size));
        } else {
            println!("Data size    : (not created yet)");
        }
    }

    // Count project envs
    let envs = find_project_envs();
    println!("Project envs : {} found under home", envs.len());
    for env in &envs {
        let size = dir_size(env);
        println!("               {} ({})", env.display(), human_size(size));
    }

    println!();
    Ok(())
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += dir_size(&p);
            }
        }
    }
    total
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
