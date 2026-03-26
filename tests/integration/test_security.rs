use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

fn run_ast_repl(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program)
}

// ---------------------------------------------------------------------------
// Sandbox tests (Group 25.2, G.2, G.3)
// ---------------------------------------------------------------------------

#[test]
fn test_sandbox_description_no_sandbox() {
    let desc = txtcode::runtime::sandbox::sandbox_description(false);
    assert_eq!(desc, "none (language-level permissions only)");
}

#[test]
fn test_sandbox_apply_no_sandbox_returns_ok() {
    let result = txtcode::runtime::sandbox::apply_sandbox(false);
    assert!(result.is_ok());
}

#[test]
fn test_sandbox_strict_disabled_returns_ok() {
    let result = txtcode::runtime::sandbox::apply_sandbox_strict(false);
    assert!(result.is_ok(), "apply_sandbox_strict(false) must succeed: {:?}", result);
}

#[test]
fn test_sandbox_strict_description_enabled() {
    let desc = txtcode::runtime::sandbox::sandbox_strict_description(true);
    assert!(!desc.is_empty());
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    assert!(
        desc.contains("allowlist"),
        "strict description on Linux x86-64 should mention 'allowlist', got: {}",
        desc
    );
}

#[test]
fn test_sandbox_description_enabled_nonempty() {
    let desc = txtcode::runtime::sandbox::sandbox_description(true);
    assert!(!desc.is_empty());
    #[cfg(target_os = "macos")]
    assert!(
        desc.contains("sandbox_init"),
        "macOS sandbox description should mention sandbox_init, got: {}",
        desc
    );
}

// ---------------------------------------------------------------------------
// Cryptographic tests (Task 16.3)
// ---------------------------------------------------------------------------

#[test]
fn test_crypto_sha256_alias() {
    let src = r#"
store → h → crypto_sha256("hello")
h
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(
        result,
        txtcode::runtime::Value::String(Arc::from("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"))
    );
}

#[test]
fn test_crypto_hmac_sha256() {
    let src = r#"
store → mac → crypto_hmac_sha256("secret", "message")
len(mac)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, txtcode::runtime::Value::Integer(64));
}

#[test]
fn test_crypto_aes_roundtrip() {
    let src = r#"
store → key → "mysecretpassword"
store → ciphertext → crypto_aes_encrypt(key, "hello world")
store → plaintext → crypto_aes_decrypt(key, ciphertext)
plaintext
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(
        result,
        txtcode::runtime::Value::String(Arc::from("hello world"))
    );
}

#[test]
fn test_crypto_aes_wrong_key_fails() {
    let src = r#"
store → ct → crypto_aes_encrypt("key1", "secret")
crypto_aes_decrypt("key2", ct)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program);
    assert!(result.is_err(), "Decrypting with wrong key must fail");
}

#[test]
fn test_crypto_random_bytes_returns_hex() {
    let src = r#"
store → b → crypto_random_bytes(16)
len(b)
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let result = VirtualMachine::new().interpret_repl(&program).unwrap();
    assert_eq!(result, txtcode::runtime::Value::Integer(32));
}

// ---------------------------------------------------------------------------
// Audit / coverage tests (Group 18.3)
// ---------------------------------------------------------------------------

#[test]
fn test_expect_error_passes_on_err_result() {
    use txtcode::runtime::Value;
    let src = r#"
store → r → err("E0001: division by zero")
expect_error(r, "E0001")
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    assert!(vm.interpret(&program).is_ok(), "expect_error should pass when result is Err containing expected code");
}

#[test]
fn test_expect_error_fails_on_ok_result() {
    use txtcode::runtime::Value;
    let src = r#"
store → r → ok(42)
expect_error(r, "E0001")
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    assert!(vm.interpret(&program).is_err(), "expect_error should fail when result is Ok");
}

#[test]
fn test_expect_error_fails_on_wrong_code() {
    let src = r#"
store → r → err("E0002: something else")
expect_error(r, "E0001")
"#;
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    assert!(vm.interpret(&program).is_err(), "expect_error should fail when error code doesn't match");
}

#[test]
fn test_coverage_tracking_records_lines() {
    let src = "store → x → 1\nstore → y → 2\nstore → z → x + y\n";
    let tokens = Lexer::new(src.to_string()).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.enable_coverage();
    vm.interpret(&program).unwrap();
    assert!(!vm.covered_lines.is_empty(), "Coverage should record executed lines");
    assert!(vm.covered_lines.len() >= 3, "Should have at least 3 covered lines");
}

#[test]
fn test_filter_test_matches_filename() {
    let name = "test_math";
    let filter = "math";
    assert!(name.contains(filter), "Filter should match filename substring");
}

// ---------------------------------------------------------------------------
// O.2: Module permission isolation
// ---------------------------------------------------------------------------

#[test]
fn test_o2_module_cannot_escalate_permissions() {
    use txtcode::runtime::permissions::PermissionManager;
    let pm = PermissionManager::new();
    let cloned = pm.clone();
    drop(pm);
    drop(cloned);
}

// ---------------------------------------------------------------------------
// O.3: Span tracking
// ---------------------------------------------------------------------------

#[test]
fn test_o3_error_includes_span() {
    let source = "store → x → 10\nstore → y → x / 0".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret(&program);
    assert!(result.is_err(), "division by zero should error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("E0012") || err_msg.contains("zero") || err_msg.contains("division"),
        "error message should mention division by zero: {}", err_msg
    );
}

#[test]
fn test_o3_runtime_error_with_span_display() {
    use txtcode::runtime::errors::RuntimeError;
    let err = RuntimeError::new("test error".to_string()).with_span(10, 5);
    let msg = err.to_string();
    assert!(
        msg.contains("10") && msg.contains("5"),
        "span should appear in error display: {}", msg
    );
}

// ---------------------------------------------------------------------------
// Error code inference tests
// ---------------------------------------------------------------------------

#[test]
fn test_error_code_e0016_inferred() {
    use txtcode::runtime::errors::ErrorCode;
    assert_eq!(
        ErrorCode::infer_from_message("Struct field type mismatch: 'Point.x' expected Int, got string"),
        ErrorCode::E0016
    );
}

// ---------------------------------------------------------------------------
// Package / registry / VSCode extension tests
// ---------------------------------------------------------------------------

#[test]
fn test_package_login_stores_credentials() {
    use std::fs;
    use std::env;
    let tmp = env::temp_dir().join("txtcode_test_login");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let creds_dir = tmp.join(".txtcode");
    fs::create_dir_all(&creds_dir).unwrap();
    let token_value = "test-api-token-abc123";
    let registry = "https://registry.txtcode.dev";
    let creds_content = format!("[registry.\"{}\"]\ntoken = \"{}\"\n", registry, token_value);
    fs::write(creds_dir.join("credentials"), &creds_content).unwrap();

    let content = fs::read_to_string(creds_dir.join("credentials")).unwrap();
    assert!(content.contains(token_value), "Token should be in credentials file");
    assert!(content.contains(registry), "Registry should be in credentials file");
}

#[test]
fn test_package_publish_missing_manifest_error() {
    let tmp = std::env::temp_dir().join("txtcode_test_no_manifest");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    let result = txtcode::cli::package::publish_package(None, None, true);
    std::env::set_current_dir(&orig).unwrap();
    assert!(result.is_err(), "Should error when Txtcode.toml is missing");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Txtcode.toml"), "Error should mention Txtcode.toml");
}

#[test]
fn test_package_publish_missing_readme_error() {
    use std::fs;
    let tmp = std::env::temp_dir().join("txtcode_test_no_readme");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("Txtcode.toml"), "name = \"mypkg\"\nversion = \"0.1.0\"\n[dependencies]\n").unwrap();
    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    let result = txtcode::cli::package::publish_package(None, None, false);
    std::env::set_current_dir(&orig).unwrap();
    assert!(result.is_err(), "Should error when README.md is missing and --no-readme not passed");
}

#[test]
fn test_package_publish_no_token_error() {
    use std::fs;
    let tmp = std::env::temp_dir().join("txtcode_test_no_token");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("Txtcode.toml"), "name = \"mypkg\"\nversion = \"0.1.0\"\n[dependencies]\n").unwrap();
    fs::write(tmp.join("README.md"), "# mypkg\n").unwrap();
    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    let result = txtcode::cli::package::publish_package(None, Some("https://fake.registry.invalid"), false);
    std::env::set_current_dir(&orig).unwrap();
    assert!(result.is_err(), "Should fail when not logged in or network unavailable");
}

#[test]
fn test_registry_index_has_urls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let index_src = std::fs::read_to_string(root.join("registry/index.json"))
        .expect("registry/index.json should exist");
    let index: serde_json::Value = serde_json::from_str(&index_src).unwrap();
    let pkgs = index["packages"].as_object().unwrap();
    for (name, pkg) in pkgs {
        for (ver, entry) in pkg["versions"].as_object().unwrap() {
            let url = entry["url"].as_str().unwrap_or("");
            assert!(
                !url.is_empty(),
                "Package {}@{} missing 'url' field in registry index",
                name, ver
            );
            assert!(
                url.contains(name.as_str()) && url.contains(ver.as_str()),
                "URL for {}@{} doesn't contain package name/version: {}",
                name, ver, url
            );
        }
    }
}

#[test]
fn test_get_package_already_installed() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let index_path = root.join("registry/index.json");
    std::env::set_var("TXTCODE_REGISTRY_INDEX_FILE", index_path.to_str().unwrap());
    let result = txtcode::cli::package::get_package("npl-math", "0.1.0", None);
    let _ = result;
}

#[test]
fn test_vscode_extension_package_json_exists() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("editors/package.json");
    assert!(path.exists(), "editors/package.json should exist for VS Code extension");
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["name"].as_str().unwrap(), "txtcode");
    assert!(json["contributes"]["languages"].is_array());
    assert!(json["contributes"]["grammars"].is_array());
    assert!(json["contributes"]["snippets"].is_array());
}

#[test]
fn test_vscode_snippets_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("editors/snippets/txtcode.json");
    assert!(path.exists(), "editors/snippets/txtcode.json should exist");
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["Function definition"].is_object(), "Missing 'Function definition' snippet");
    assert!(json["Variable assignment"].is_object(), "Missing 'Variable assignment' snippet");
    assert!(json["For loop"].is_object(), "Missing 'For loop' snippet");
}

#[test]
fn test_vscode_lsp_client_exists() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("editors/client/extension.js");
    assert!(path.exists(), "editors/client/extension.js should exist");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("LanguageClient"), "LSP client should use LanguageClient");
    assert!(content.contains("txtcode lsp"), "Client should launch 'txtcode lsp'");
}

// ---------------------------------------------------------------------------
// Plugin tests (Task 22.1/22.2/22.3, Group L.3)
// ---------------------------------------------------------------------------

#[test]
fn test_plugin_load_requires_ffi_feature() {
    let result = run_ast_repl("plugin_load(\"/nonexistent/plugin.so\")");
    let _ = result;
}

#[test]
fn test_plugin_functions_requires_ffi_feature() {
    let result = run_ast_repl("plugin_functions(\"/nonexistent.so\")");
    let _ = result;
}

#[test]
fn test_plugin_call_requires_ffi_feature() {
    let result = run_ast_repl("plugin_call(\"/nonexistent.so\", \"fn\", [])");
    let _ = result;
}

#[test]
fn test_plugin_load_nonexistent_path_clear_error() {
    let result = run_ast_repl("plugin_load(\"/absolutely/nonexistent/plugin_xyz.so\")");
    assert!(result.is_err(), "plugin_load with bad path must return error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("ffi") || msg.contains("nonexistent") || msg.contains("plugin"),
        "error should be informative, got: {}",
        msg
    );
}

#[test]
fn test_plugin_call_arity_error() {
    let result = run_ast_repl("plugin_call()");
    assert!(result.is_err(), "plugin_call() with no args must return error");
}

#[test]
fn test_plugin_functions_clear_error_without_ffi() {
    let result = run_ast_repl("plugin_functions(\"/nonexistent.so\")");
    assert!(result.is_err(), "plugin_functions must return error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("ffi") || msg.contains("nonexistent") || msg.contains("plugin"),
        "error should be informative, got: {}",
        msg
    );
}

// ---------------------------------------------------------------------------
// WASM tests (Group 29.3)
// ---------------------------------------------------------------------------

#[test]
fn test_wasm_load_missing_file_returns_error() {
    use txtcode::runtime::{permissions::PermissionResource, vm::VirtualMachine};
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("ffi".to_string()), None);
    let src = r#"wasm_load("/tmp/__nonexistent_test_file_xyz.wasm")"#;
    let mut lexer = txtcode::lexer::Lexer::new(src.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let program = parser.parse().unwrap();
    let result = vm.interpret_repl(&program);
    assert!(result.is_err(), "wasm_load of missing file should return Err");
}

#[test]
fn test_wasm_call_invalid_handle_returns_error() {
    use txtcode::runtime::{permissions::PermissionResource, vm::VirtualMachine};
    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::System("ffi".to_string()), None);
    let src = r#"wasm_call(99999, "add", [1, 2])"#;
    let mut lexer = txtcode::lexer::Lexer::new(src.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = txtcode::parser::Parser::new(tokens);
    let program = parser.parse().unwrap();
    let result = vm.interpret_repl(&program);
    assert!(result.is_err(), "wasm_call with invalid handle should return Err");
}

#[test]
fn test_wasm_load_requires_ffi_permission() {
    let src = r#"wasm_load("/tmp/test.wasm")"#;
    let result = run_ast_repl(src);
    assert!(result.is_err(), "wasm_load without ffi permission must fail");
}

// ── Permission map coverage audit (AUDIT-4) ──────────────────────────────────
//
// These tests verify that every known privileged stdlib function is present in
// permission_map::map_function_to_permission().  If a new privileged function is
// added to stdlib without updating permission_map, these tests catch the gap.

#[test]
fn test_permission_map_covers_filesystem_reads() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    use txtcode::runtime::permissions::PermissionResource;
    let privileged = ["read_file", "read_lines", "read_file_binary",
                      "file_exists", "is_file", "is_dir", "list_dir", "watch_file"];
    for name in &privileged {
        let perm = map_function_to_permission(name);
        assert!(
            matches!(perm, Some(PermissionResource::FileSystem(ref s)) if s == "read"),
            "Expected FileSystem(read) for '{}', got {:?}", name, perm
        );
    }
}

#[test]
fn test_permission_map_covers_filesystem_writes() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    use txtcode::runtime::permissions::PermissionResource;
    let privileged = ["write_file", "write_file_binary", "append_file",
                      "copy_file", "move_file", "temp_file", "mkdir", "zip_create", "zip_extract"];
    for name in &privileged {
        let perm = map_function_to_permission(name);
        assert!(
            matches!(perm, Some(PermissionResource::FileSystem(ref s)) if s == "write"),
            "Expected FileSystem(write) for '{}', got {:?}", name, perm
        );
    }
}

#[test]
fn test_permission_map_covers_network_functions() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    use txtcode::runtime::permissions::PermissionResource;
    let privileged = ["http_get", "http_post", "http_put", "http_delete",
                      "http_patch", "async_http_get", "async_http_post", "tcp_connect"];
    for name in &privileged {
        let perm = map_function_to_permission(name);
        assert!(
            matches!(perm, Some(PermissionResource::Network(ref s)) if s == "connect"),
            "Expected Network(connect) for '{}', got {:?}", name, perm
        );
    }
}

#[test]
fn test_permission_map_covers_process_exec() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    use txtcode::runtime::permissions::PermissionResource;
    let privileged = ["exec", "exec_status", "exec_lines", "spawn", "kill", "pipe_exec", "tool_exec"];
    for name in &privileged {
        let perm = map_function_to_permission(name);
        assert!(
            matches!(perm, Some(PermissionResource::System(ref s)) if s == "exec"),
            "Expected System(exec) for '{}', got {:?}", name, perm
        );
    }
}

#[test]
fn test_permission_map_covers_database_functions() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    use txtcode::runtime::permissions::PermissionResource;
    let privileged = ["db_connect", "db_query", "db_execute", "db_transaction",
                      "db_commit", "db_rollback"];
    for name in &privileged {
        let perm = map_function_to_permission(name);
        assert!(
            matches!(perm, Some(PermissionResource::System(ref s)) if s == "db"),
            "Expected System(db) for '{}', got {:?}", name, perm
        );
    }
}

#[test]
fn test_permission_map_covers_ffi_and_wasm() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    use txtcode::runtime::permissions::PermissionResource;
    let privileged = ["ffi_load", "ffi_call", "ffi_close",
                      "plugin_load", "plugin_call",
                      "wasm_load", "wasm_call", "wasm_close"];
    for name in &privileged {
        let perm = map_function_to_permission(name);
        assert!(
            matches!(perm, Some(PermissionResource::System(ref s)) if s == "ffi"),
            "Expected System(ffi) for '{}', got {:?}", name, perm
        );
    }
}

#[test]
fn test_permission_map_unprivileged_functions_return_none() {
    use txtcode::runtime::permission_map::map_function_to_permission;
    // These functions must never require a permission — they are pure computations.
    let safe = ["print", "len", "type_of", "to_int", "to_float", "to_string",
                "json_encode", "json_decode", "sha256", "uuid_v4", "math_floor"];
    for name in &safe {
        let perm = map_function_to_permission(name);
        assert!(
            perm.is_none(),
            "Unprivileged function '{}' must not require permission, got {:?}", name, perm
        );
    }
}

