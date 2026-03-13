//! `txtcode doctor` — environment health check, and verbose info.

use std::fs;

pub fn run_doctor() {
    let mut ok = true;

    let pass = |msg: &str| println!("  [ok]   {}", msg);
    let fail = |msg: &str| {
        println!("  [FAIL] {}", msg);
    };
    let warn = |msg: &str| println!("  [warn] {}", msg);

    println!("txtcode doctor — environment check\n");

    match std::env::current_exe() {
        Ok(path) => pass(&format!("txtcode binary: {}", path.display())),
        Err(e) => {
            fail(&format!("Cannot determine binary path: {}", e));
            ok = false;
        }
    }
    pass(&format!("version: {}", env!("CARGO_PKG_VERSION")));

    let cwd = std::env::current_dir().unwrap_or_default();
    let manifest = cwd.join("Txtcode.toml");
    if manifest.exists() {
        pass(&format!("Project manifest found: {}", manifest.display()));
    } else {
        warn("No Txtcode.toml found in current directory (run `txtcode init` to create one)");
    }

    let src_dir = cwd.join("src");
    if src_dir.exists() {
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

    match crate::config::Config::get_txtcode_home() {
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
                    warn(&format!(
                        "{}/: not found (will be created on first use)",
                        subdir
                    ));
                }
            }
        }
        Err(e) => {
            fail(&format!("Cannot resolve txtcode home: {}", e));
            ok = false;
        }
    }

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

    let tmp = std::env::temp_dir();
    let probe = tmp.join("txtcode_doctor_probe");
    match fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            pass(&format!("temp directory writable: {}", tmp.display()));
        }
        Err(e) => {
            fail(&format!(
                "temp directory not writable ({}): {}",
                tmp.display(),
                e
            ));
            ok = false;
        }
    }

    println!();
    if ok {
        println!("All checks passed.");
    } else {
        println!("Some checks failed — see [FAIL] items above.");
        std::process::exit(1);
    }
}

pub fn print_verbose_info() {
    let version = env!("CARGO_PKG_VERSION");
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!("Txt-code v{}", version);
    println!("Build: {}", build);
    println!("Platform: {}-{}", os, arch);
}
