use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// txtcode self uninstall
// ─────────────────────────────────────────────────────────────────────────────

pub fn self_uninstall(yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════╗");
    println!("║     Txt-code Uninstall                   ║");
    println!("╚══════════════════════════════════════════╝\n");

    // Locate the running binary
    let bin_path =
        std::env::current_exe().map_err(|e| format!("Cannot locate txtcode binary: {}", e))?;

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
    println!(
        "     Removes : {}  (config, cache, logs, global packages)",
        global_data.display()
    );
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
            Ok(n) if (1..=3).contains(&n) => n,
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

    // Check for other copies of the binary in standard locations
    let other_copies = find_other_binaries(&bin_path);
    if !other_copies.is_empty() {
        println!("\nAlso found in other locations:");
        for p in &other_copies {
            println!("  - {}", p.display());
        }
        print!("Remove these too? [y/N]: ");
        io::stdout().flush()?;
        let mut ans = String::new();
        io::stdin().read_line(&mut ans)?;
        if ans.trim().eq_ignore_ascii_case("y") {
            for p in &other_copies {
                match fs::remove_file(p) {
                    Ok(_) => println!("  ✓ Removed {}", p.display()),
                    Err(e) => eprintln!("  ✗ Could not remove {}: {}", p.display(), e),
                }
            }
        }
    }

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

/// Find other copies of the txtcode binary in standard install locations,
/// excluding the one currently running.
fn find_other_binaries(current: &PathBuf) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        PathBuf::from("/usr/local/bin/txtcode"),
        PathBuf::from("/usr/bin/txtcode"),
        home.join(".local/bin/txtcode"),
        home.join(".cargo/bin/txtcode"),
    ];
    candidates
        .into_iter()
        .filter(|p| p.exists() && p != current)
        .collect()
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
    if depth > 8 {
        return;
    } // don't go too deep
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
        ".bashrc",
        ".bash_profile",
        ".zshrc",
        ".zprofile",
        ".profile",
    ];

    let marker = "# txtcode";

    for filename in &shell_files {
        let path = home.join(filename);
        if !path.exists() {
            continue;
        }
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
fn remove_binary_deferred(bin_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        // Include PID in temp name to prevent predictable-path races (TOCTOU)
        let tmp = std::env::temp_dir()
            .join(format!("txtcode_remove_{}.sh", std::process::id()));
        let script = format!(
            "#!/bin/sh\nsleep 1\nrm -f \"{}\"\nrm -f \"$0\"\n",
            bin_path.display()
        );
        fs::write(&tmp, &script)?;
        // Make executable
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o700))?;
        // Run in background and exit
        std::process::Command::new("sh").arg(&tmp).spawn()?;
    }
    #[cfg(not(unix))]
    {
        // On Windows, rename binary so it's unlocked, then remove
        let tmp_path = bin_path.with_extension("old");
        let _ = fs::rename(bin_path, &tmp_path);
        // Schedule deletion - best effort
        std::process::Command::new("cmd")
            .args([
                "/C",
                "ping",
                "-n",
                "2",
                "127.0.0.1",
                ">nul",
                "&",
                "del",
                &tmp_path.to_string_lossy(),
            ])
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
    print!("Checking for updates... ");
    io::stdout().flush()?;

    let latest_version = match fetch_latest_version() {
        Ok(v) => {
            println!("done.");
            v
        }
        Err(_) => {
            println!("failed.");
            println!("Could not reach GitHub. Check your connection or visit:");
            println!("  https://github.com/iga2x/Txtcode/releases");
            return Ok(());
        }
    };

    println!("Latest version  : v{}", latest_version);

    // Compare using semver
    let cur = semver::Version::parse(current_version).unwrap_or(semver::Version::new(0, 0, 0));
    let lat = semver::Version::parse(&latest_version).unwrap_or(semver::Version::new(0, 0, 0));

    if lat <= cur {
        println!("\nYou are already on the latest version.");
        return Ok(());
    }

    println!(
        "\nNew version available: v{} → v{}",
        current_version, latest_version
    );

    // Detect platform label (matches release asset naming)
    let label = match detect_platform_label() {
        Ok(l) => l,
        Err(e) => {
            println!("\nCannot auto-update on this platform: {}", e);
            println!("Download manually: https://github.com/iga2x/Txtcode/releases");
            return Ok(());
        }
    };

    // Build URL for the bare binary asset
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let filename = format!("txtcode-{}-{}{}", latest_version, label, ext);
    let url = format!(
        "https://github.com/iga2x/Txtcode/releases/download/v{}/{}",
        latest_version, filename
    );

    println!("Downloading {} ...", filename);

    let tmp = std::env::temp_dir().join(&filename);
    if let Err(e) = download_file(&url, &tmp) {
        println!("Download failed: {}", e);
        println!("Download manually: https://github.com/iga2x/Txtcode/releases");
        return Ok(());
    }

    // Download and verify signature + SHA-256 checksum before replacing the binary (0.8)
    let sig_url = format!(
        "https://github.com/iga2x/Txtcode/releases/download/v{}/{}.sig",
        latest_version, filename
    );
    let sha256_url = format!(
        "https://github.com/iga2x/Txtcode/releases/download/v{}/sha256sums",
        latest_version
    );

    let binary_bytes = fs::read(&tmp).map_err(|e| format!("Failed to read downloaded binary: {}", e))?;

    // Verify Ed25519 signature — mandatory, never skipped
    print!("Verifying binary signature... ");
    io::stdout().flush()?;
    let sig_tmp = std::env::temp_dir().join(format!("{}.sig", filename));
    if let Err(e) = download_file(&sig_url, &sig_tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(format!(
            "Could not download signature file for v{}: {}. \
             Update aborted — refusing to install unsigned binary. \
             Download manually from: https://github.com/iga2x/Txtcode/releases",
            latest_version, e
        ).into());
    }
    let sig_bytes = fs::read(&sig_tmp).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&sig_tmp);
        format!("Could not read signature file: {}", e)
    })?;
    let _ = fs::remove_file(&sig_tmp);
    if let Err(e) = crate::security::update_verifier::verify_update_binary(&binary_bytes, &sig_bytes) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Signature verification failed: {}", e).into());
    }
    println!("OK");

    // Verify SHA-256 checksum — mandatory, never skipped
    print!("Verifying SHA-256 checksum... ");
    io::stdout().flush()?;
    let sha256_tmp = std::env::temp_dir().join(format!("sha256sums_{}", latest_version));
    if let Err(e) = download_file(&sha256_url, &sha256_tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(format!(
            "Could not download sha256sums for v{}: {}. \
             Update aborted — refusing to install unverified binary.",
            latest_version, e
        ).into());
    }
    let sums_content = fs::read_to_string(&sha256_tmp).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&sha256_tmp);
        format!("Could not read sha256sums: {}", e)
    })?;
    let _ = fs::remove_file(&sha256_tmp);
    if let Err(e) = crate::security::update_verifier::verify_sha256(&sums_content, &filename, &binary_bytes) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Checksum verification failed: {}", e).into());
    }
    println!("OK");

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    }

    let current_bin = std::env::current_exe()?;
    println!("Replacing: {}", current_bin.display());

    // Check if we can write to the current binary location
    if let Err(e) = fs::OpenOptions::new().write(true).open(&current_bin) {
        println!("\nNo write permission to {}: {}", current_bin.display(), e);
        println!("Try: sudo txtcode self update");
        let _ = fs::remove_file(&tmp);
        return Ok(());
    }

    apply_update_deferred(&current_bin, &tmp)?;

    println!(
        "\nUpdate scheduled. Restart txtcode to use v{}.",
        latest_version
    );
    Ok(())
}

fn detect_platform_label() -> Result<String, Box<dyn std::error::Error>> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        other => return Err(format!("unsupported OS '{}'", other).into()),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => return Err(format!("unsupported arch '{}'", other).into()),
    };
    Ok(format!("{}-{}", os, arch))
}

fn download_file(url: &str, dest: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "net"))]
    {
        let _ = (url, dest);
        return Err(
            "Downloading updates requires the 'net' feature. \
             Rebuild with: cargo build --features net"
                .into(),
        );
    }
    #[cfg(feature = "net")]
    {
        use std::io::Write as IoWrite;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .user_agent(format!("txtcode/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let resp = client.get(url).send()?;
        if !resp.status().is_success() {
            return Err(format!("server returned HTTP {}", resp.status()).into());
        }

        let bytes = resp.bytes()?;
        let mut file = fs::File::create(dest)?;
        file.write_all(&bytes)?;
        Ok(())
    }
}

#[cfg(unix)]
fn apply_update_deferred(current: &Path, new_bin: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Include PID in temp name to prevent predictable-path races (TOCTOU)
    let tmp_script = std::env::temp_dir()
        .join(format!("txtcode_update_{}.sh", std::process::id()));
    let script = format!(
        "#!/bin/sh\nsleep 1\ncp -f \"{}\" \"{}\"\nchmod 755 \"{}\"\nrm -f \"$0\"\n",
        new_bin.display(),
        current.display(),
        current.display()
    );
    fs::write(&tmp_script, &script)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tmp_script, fs::Permissions::from_mode(0o700))?;
    std::process::Command::new("sh").arg(&tmp_script).spawn()?;
    Ok(())
}

#[cfg(not(unix))]
fn apply_update_deferred(
    current: &PathBuf,
    new_bin: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // Include PID in temp name to prevent predictable-path races (TOCTOU)
    let tmp_script = std::env::temp_dir()
        .join(format!("txtcode_update_{}.bat", std::process::id()));
    let script = format!(
        "@echo off\nping -n 2 127.0.0.1 >nul\ncopy /Y \"{}\" \"{}\" >nul\ndel \"%~f0\"\n",
        new_bin.display(),
        current.display()
    );
    fs::write(&tmp_script, &script)?;
    std::process::Command::new("cmd")
        .args(["/C", &tmp_script.to_string_lossy()])
        .spawn()?;
    Ok(())
}

fn fetch_latest_version() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(not(feature = "net"))]
    return Err(
        "Checking for updates requires the 'net' feature. \
         Rebuild with: cargo build --features net"
            .into(),
    );

    #[cfg(feature = "net")]
    {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("txtcode/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let resp = client
            .get("https://api.github.com/repos/iga2x/Txtcode/releases/latest")
            .send()?;

        let json: serde_json::Value = resp.json()?;
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
    }
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
