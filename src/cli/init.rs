//! `txtcode init` — scaffold a new project.

use std::fs;

pub fn init_project(name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let project_name = name.map(|n| n.to_string()).unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string()
    });

    let project_dir = if let Some(n) = name {
        let d = cwd.join(n);
        fs::create_dir_all(&d)?;
        d
    } else {
        cwd.clone()
    };

    // W.4: generate txtcode.toml with [project] table so Builder::run() can
    // discover and load it via ProjectConfig::discover_near().
    let toml_path = project_dir.join("txtcode.toml");
    if !toml_path.exists() {
        fs::write(
            &toml_path,
            format!(
                r#"[project]
name = "{name}"
version = "0.1.0"
entry = "src/main.tc"
backend = "ast"

[dependencies]
"#,
                name = project_name
            ),
        )?;
        println!("  created  txtcode.toml");
    }

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
├── txtcode.toml   # Project manifest
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
