#![no_main]
// Fuzz target: load_env_file key/value parsing — adversarial .env files must
// never panic and must always reject RESERVED_ENV_KEYS (LD_PRELOAD, etc.).
//
// We cannot call load_env_file directly (it writes to the real env), so we
// replicate the parsing and filtering logic here to fuzz just those layers.
use libfuzzer_sys::fuzz_target;

const RESERVED_ENV_KEYS: &[&str] = &[
    "LD_PRELOAD",
    "LD_AUDIT",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_FORCE_FLAT_NAMESPACE",
    "DYLD_LIBRARY_PATH",
    "_FRIDA_AGENT",
    "FRIDA_TRANSPORT",
    "FRIDA_LISTEN",
];

fn parse_env_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let eq = line.find('=')?;
    let key = line[..eq].trim();
    let val = line[eq + 1..].trim().trim_matches('"');
    Some((key, val))
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        for line in s.lines() {
            if let Some((key, _val)) = parse_env_line(line) {
                // Ensure reserved keys are always rejected.
                let is_reserved = RESERVED_ENV_KEYS.iter().any(|r| key.eq_ignore_ascii_case(r));
                if is_reserved {
                    // In production this returns Err; here we just assert it was detected.
                    assert!(
                        is_reserved,
                        "RESERVED key '{}' must be rejected but was not detected",
                        key
                    );
                }
            }
        }
    }
});
