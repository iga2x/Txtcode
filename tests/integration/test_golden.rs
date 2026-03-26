//! T.2 — Golden tests for DSL behavior.
//!
//! Each `.tc` program in `tests/golden/programs/` has a matching `.txt` file
//! in `tests/golden/expected/`.  The expected file has two formats:
//!
//! - **Exact match**: lines are compared verbatim against stdout.
//! - **Pattern match**: lines beginning with `EXIT_CODE:` or `CONTAINS:` are
//!   directives; the rest are ignored.
//!
//! Pattern format:
//! ```
//! EXIT_CODE: 1
//! CONTAINS: some error substring
//! ```
//!
//! Only `EXIT_CODE` and `CONTAINS` directives are tested; plain-text expected
//! files are matched exactly against stdout (trimmed).

use std::path::{Path, PathBuf};
use std::process::Command;

// ── Runner ────────────────────────────────────────────────────────────────────

struct GoldenResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

fn run_golden(program: &str) -> GoldenResult {
    let binary = env!("CARGO_BIN_EXE_txtcode");
    let manifest = env!("CARGO_MANIFEST_DIR");
    let program_path = PathBuf::from(manifest)
        .join("tests/golden/programs")
        .join(program);

    let output = Command::new(binary)
        .args(["run", program_path.to_str().unwrap()])
        .current_dir(manifest)
        .output()
        .unwrap_or_else(|e| panic!("failed to run txtcode binary: {}", e));

    GoldenResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

fn check_golden(program: &str, expected_file: &str) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let expected_path = Path::new(manifest)
        .join("tests/golden/expected")
        .join(expected_file);

    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("cannot read expected file '{}': {}", expected_path.display(), e));

    let result = run_golden(program);

    // Detect pattern format: any line starting with EXIT_CODE: or CONTAINS:
    let is_pattern = expected.lines().any(|l| {
        l.starts_with("EXIT_CODE:") || l.starts_with("CONTAINS:")
    });

    if is_pattern {
        // Process directives
        for line in expected.lines() {
            if let Some(code_str) = line.strip_prefix("EXIT_CODE:") {
                let expected_code: i32 = code_str.trim().parse()
                    .unwrap_or_else(|_| panic!("invalid EXIT_CODE in {}", expected_file));
                assert_eq!(
                    result.exit_code, expected_code,
                    "golden '{}': exit code mismatch\nstdout: {}\nstderr: {}",
                    program, result.stdout, result.stderr,
                );
            } else if let Some(needle) = line.strip_prefix("CONTAINS:") {
                let needle = needle.trim();
                let combined = format!("{}{}", result.stdout, result.stderr);
                assert!(
                    combined.contains(needle),
                    "golden '{}': output does not contain {:?}\nstdout: {}\nstderr: {}",
                    program, needle, result.stdout, result.stderr,
                );
            }
        }
    } else {
        // Exact match against stdout (trimmed)
        assert_eq!(
            result.exit_code, 0,
            "golden '{}': expected exit 0, got {}\nstdout: {}\nstderr: {}",
            program, result.exit_code, result.stdout, result.stderr,
        );
        let actual = result.stdout.trim_end_matches('\n');
        let expected_trimmed = expected.trim_end_matches('\n');
        assert_eq!(
            actual, expected_trimmed,
            "golden '{}': stdout mismatch\nexpected:\n{}\n\nactual:\n{}",
            program, expected_trimmed, actual,
        );
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[test]
fn golden_hello_world() {
    check_golden("hello_world.tc", "hello_world.txt");
}

#[test]
fn golden_fibonacci() {
    check_golden("fibonacci.tc", "fibonacci.txt");
}

#[test]
fn golden_permission_denied() {
    check_golden("permission_denied.tc", "permission_denied.txt");
}

#[test]
fn golden_duplicate_function() {
    check_golden("duplicate_function.tc", "duplicate_function.txt");
}

#[test]
fn golden_undefined_variable() {
    check_golden("undefined_variable.tc", "undefined_variable.txt");
}
