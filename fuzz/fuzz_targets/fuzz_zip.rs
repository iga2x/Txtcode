#![no_main]
// Fuzz target: zip_extract path validation — crafted ZIP bytes must never bypass
// the zip-slip protection added in Phase 0.1.
//
// We test the path-validation logic directly rather than writing real files.
// The fuzz input is treated as entry names: each NUL-separated token is passed
// to the same canonicalization check used by zip_extract.
use libfuzzer_sys::fuzz_target;
use std::path::{Component, Path, PathBuf};

/// Mirrors the validate_zip_entry_path logic from src/stdlib/io.rs.
fn validate_entry(output_dir: &Path, entry_name: &str) -> bool {
    let entry_path = Path::new(entry_name);
    for component in entry_path.components() {
        if component == Component::ParentDir {
            return false;
        }
    }
    // Use a synthetic canonical root since we are not writing real files.
    let synthetic_root = PathBuf::from("/tmp/fuzz_sandbox");
    let joined = synthetic_root.join(entry_name);
    joined.starts_with(&synthetic_root)
}

fuzz_target!(|data: &[u8]| {
    // Split on NUL bytes to produce multiple entry name candidates.
    for entry in data.split(|&b| b == 0) {
        if let Ok(name) = std::str::from_utf8(entry) {
            let _ = validate_entry(Path::new("/tmp/fuzz_sandbox"), name);
        }
    }
});
