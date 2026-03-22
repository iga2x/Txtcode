/// W.5: Wire tests/tc/*.tc files into `cargo test` so CI runs them automatically.
/// Each test spawns the `txtcode` binary and asserts clean exit + no assertion failures.

fn run_tc_file(path: &str) {
    let binary = env!("CARGO_BIN_EXE_txtcode");
    let project_root = env!("CARGO_MANIFEST_DIR");
    let output = std::process::Command::new(binary)
        .args(["run", path])
        .current_dir(project_root)
        .output()
        .unwrap_or_else(|e| panic!("failed to run txtcode binary: {}", e));

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Fail if the process exited with non-zero
    assert!(
        output.status.success(),
        "tc test '{}' failed (exit {})\nstdout: {}\nstderr: {}",
        path,
        output.status.code().unwrap_or(-1),
        stdout,
        stderr
    );

    // Fail if any assertion message appears in stdout or stderr
    assert!(
        !stdout.contains("ASSERTION FAILED") && !stderr.contains("ASSERTION FAILED"),
        "assertion failed in '{}'\nstdout: {}\nstderr: {}",
        path,
        stdout,
        stderr
    );
}

#[test]
fn test_tc_arithmetic() {
    run_tc_file("tests/tc/test_arithmetic.tc");
}

#[test]
fn test_tc_strings() {
    run_tc_file("tests/tc/test_strings.tc");
}

#[test]
fn test_tc_collections() {
    run_tc_file("tests/tc/test_collections.tc");
}

#[test]
fn test_tc_functions() {
    run_tc_file("tests/tc/test_functions.tc");
}
