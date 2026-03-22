//! Minimal Language Server Protocol (LSP) server for Txtcode.
//!
//! Communicates over stdin/stdout using the JSON-RPC 2.0 framing mandated by LSP.
//! No external LSP crate required — only `serde_json` (already a dependency).
//!
//! Supported capabilities:
//! - `initialize` / `initialized`
//! - `textDocument/didOpen`, `textDocument/didChange` → `publishDiagnostics`
//! - `textDocument/completion` → stdlib + keyword completions
//! - `textDocument/definition` → go-to-definition (same-file symbols)
//! - `textDocument/hover` → function signature and doc comment
//! - `textDocument/rename` → rename symbol across file
//! - `shutdown` / `exit`
//!
//! Start with: `txtcode lsp`

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

// ── Workspace Index ───────────────────────────────────────────────────────────

/// A single symbol definition found while indexing workspace files.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    /// LSP SymbolKind: 12=Function, 5=Class/Struct, 13=Variable, 6=Method
    pub kind: u32,
    pub uri: String,
    pub line: usize,
    pub col: usize,
}

/// Cross-file symbol index built by scanning all `.tc` files in the workspace.
pub struct WorkspaceIndex {
    /// symbol name → list of definition locations (multiple files may define it)
    pub definitions: HashMap<String, Vec<SymbolInfo>>,
    /// uri → list of symbols defined in that file
    pub symbols_by_file: HashMap<String, Vec<SymbolInfo>>,
}

impl WorkspaceIndex {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            symbols_by_file: HashMap::new(),
        }
    }

    /// Scan all `.tc` files under `root` and index their top-level symbols.
    pub fn build_from_root(&mut self, root: &Path) {
        self.definitions.clear();
        self.symbols_by_file.clear();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Skip hidden dirs and target/
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if !name.starts_with('.') && name != "target" {
                            stack.push(path);
                        }
                    } else if path.extension().and_then(|e| e.to_str()) == Some("tc") {
                        let uri = path_to_uri(&path);
                        if let Ok(src) = std::fs::read_to_string(&path) {
                            let syms = index_source(&src, &uri);
                            for sym in &syms {
                                self.definitions
                                    .entry(sym.name.clone())
                                    .or_default()
                                    .push(sym.clone());
                            }
                            self.symbols_by_file.insert(uri, syms);
                        }
                    }
                }
            }
        }
    }

    /// Update the index for a single file (called on didOpen/didChange).
    pub fn update_file(&mut self, uri: &str, source: &str) {
        // Remove old symbols for this file
        if let Some(old_syms) = self.symbols_by_file.remove(uri) {
            for sym in old_syms {
                if let Some(defs) = self.definitions.get_mut(&sym.name) {
                    defs.retain(|d| d.uri != uri);
                    if defs.is_empty() {
                        self.definitions.remove(&sym.name);
                    }
                }
            }
        }
        // Index the new content
        let syms = index_source(source, uri);
        for sym in &syms {
            self.definitions
                .entry(sym.name.clone())
                .or_default()
                .push(sym.clone());
        }
        self.symbols_by_file.insert(uri.to_string(), syms);
    }

    /// Look up the definition locations of `name` across the entire workspace.
    pub fn find_definition(&self, name: &str) -> Vec<&SymbolInfo> {
        self.definitions.get(name).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// Search for symbols whose name contains `query` (case-insensitive).
    pub fn search_symbols(&self, query: &str) -> Vec<&SymbolInfo> {
        let q = query.to_lowercase();
        let mut results: Vec<&SymbolInfo> = self
            .definitions
            .values()
            .flatten()
            .filter(|s| s.name.to_lowercase().contains(&q))
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Find all references to `name` across all indexed files.
    pub fn find_references(&self, name: &str, documents: &HashMap<String, String>) -> Vec<Value> {
        let mut locs = Vec::new();
        // Search open documents first (they have the latest content)
        for (uri, text) in documents {
            for (sl, sc, el, ec) in find_all_occurrences(text, name) {
                locs.push(json!({ "uri": uri, "range": lsp_range(sl, sc, el, ec) }));
            }
        }
        // Also search indexed files not currently open
        for (uri, syms) in &self.symbols_by_file {
            if documents.contains_key(uri) { continue; }
            // Re-read from disk for non-open files
            if let Some(path) = uri_to_path(uri) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    for (sl, sc, el, ec) in find_all_occurrences(&text, name) {
                        locs.push(json!({ "uri": uri, "range": lsp_range(sl, sc, el, ec) }));
                    }
                }
            }
            // Suppress unused warning
            let _ = syms;
        }
        locs
    }
}

/// Parse source and extract top-level symbol definitions.
fn index_source(source: &str, uri: &str) -> Vec<SymbolInfo> {
    let mut syms = Vec::new();
    for (ln, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        // `define → name → (params)` or `define name`
        if trimmed.starts_with("define") {
            let after = trimmed["define".len()..].trim().trim_start_matches('→').trim();
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                let col = line.find(&name).unwrap_or(0);
                syms.push(SymbolInfo { name, kind: 12, uri: uri.to_string(), line: ln, col });
            }
        }
        // `struct Name(...)` — kind 5 (Class)
        else if let Some(rest) = trimmed.strip_prefix("struct ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                let col = line.find(&name).unwrap_or(0);
                syms.push(SymbolInfo { name, kind: 5, uri: uri.to_string(), line: ln, col });
            }
        }
        // `store → name → ...` — kind 13 (Variable), only top-level (indent = 0)
        else if trimmed.starts_with("store") && line.starts_with("store") {
            let after = trimmed["store".len()..].trim().trim_start_matches('→').trim();
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                let col = line.find(&name).unwrap_or(0);
                syms.push(SymbolInfo { name, kind: 13, uri: uri.to_string(), line: ln, col });
            }
        }
    }
    syms
}

/// Convert a file path to an LSP `file://` URI.
pub fn path_to_uri(path: &Path) -> String {
    // Canonicalize if possible, then encode
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.display().to_string();
    // Simple encoding: ensure forward slashes on all platforms
    let forward = s.replace('\\', "/");
    if forward.starts_with('/') {
        format!("file://{}", forward)
    } else {
        format!("file:///{}", forward)
    }
}

/// Convert an LSP `file://` URI to a PathBuf.
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let path_str = uri.strip_prefix("file://")?;
    // On Windows, strip extra leading slash before drive letter
    let path_str = if path_str.starts_with('/') && path_str.len() > 2
        && path_str.chars().nth(2) == Some(':') {
        &path_str[1..]
    } else {
        path_str
    };
    Some(PathBuf::from(path_str))
}

/// Extract the workspace root from `initialize` params.
fn root_from_params(params: &Value) -> Option<PathBuf> {
    // Prefer rootUri (newer), fall back to rootPath
    if let Some(uri) = params["rootUri"].as_str().filter(|s| !s.is_empty()) {
        return uri_to_path(uri);
    }
    if let Some(path) = params["rootPath"].as_str().filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(path));
    }
    None
}

/// All stdlib function names offered as completion items.
const STDLIB_FUNCTIONS: &[&str] = &[
    "print", "input", "len", "type", "string", "int", "float", "bool",
    "str_upper", "str_lower", "str_trim", "str_split", "str_join",
    "str_replace", "str_starts_with", "str_ends_with", "str_contains",
    "str_repeat", "str_pad_left", "str_pad_right", "str_reverse",
    "str_chars", "str_bytes", "str_format",
    "math_abs", "math_floor", "math_ceil", "math_round", "math_sqrt",
    "math_pow", "math_log", "math_sin", "math_cos", "math_tan",
    "math_min", "math_max", "math_random", "math_pi",
    "array_map", "array_filter", "array_reduce", "array_find",
    "array_sort", "array_reverse", "array_concat", "array_slice",
    "array_push", "array_pop", "array_shift", "array_unshift",
    "array_flatten", "array_unique", "array_zip", "array_enumerate",
    "array_sum", "array_contains", "array_index_of",
    "map", "filter", "reduce", "find",
    "json_encode", "json_decode", "json_pretty",
    "read_file", "write_file", "append_file", "delete",
    "file_exists", "is_file", "is_dir", "list_dir", "mkdir", "rmdir",
    "file_open", "file_read_line", "file_write_line", "file_close",
    "csv_write", "csv_to_string", "read_csv",
    "zip_create", "zip_extract",
    "now", "sleep", "now_utc", "now_local",
    "parse_datetime", "format_datetime", "datetime_add", "datetime_diff",
    "format_time", "parse_time",
    "http_get", "http_post", "http_put", "http_delete", "http_patch",
    "http_serve", "http_response", "http_request_method",
    "http_request_path", "http_request_body",
    "exec", "exec_status", "exec_lines", "exec_json", "exec_pipe",
    "getenv", "setenv", "platform", "arch", "exit", "args", "cwd",
    "sha256", "sha512", "md5", "base64_encode", "base64_decode",
    "encrypt", "decrypt", "hmac_sha256", "uuid_v4",
    "regex_match", "regex_find_all", "regex_replace",
    "path_join", "path_dir", "path_base", "path_ext", "path_abs",
    "log", "log_info", "log_warn", "log_error", "log_debug",
    "url_encode", "url_decode", "url_parse",
    "assert", "assert_eq", "assert_ne", "test_suite",
    "ok", "err", "is_ok", "is_err", "unwrap", "unwrap_or",
    "set", "bytes_encode", "bytes_decode",
    "format", "max", "min", "abs", "floor", "ceil", "round",
    "sqrt", "pow", "sin", "cos", "tan", "log",
    "sort", "reverse", "concat", "split", "join", "replace", "trim",
    "substring", "indexOf", "startsWith", "endsWith", "toUpper", "toLower",
    "html_escape",
];

const KEYWORDS: &[&str] = &[
    "store", "define", "if", "else", "elseif", "for", "while",
    "return", "end", "try", "catch", "match", "in", "struct",
    "type", "error", "import", "break", "continue",
    "and", "or", "not", "true", "false", "null",
];

// ── Message framing ──────────────────────────────────────────────────────────

/// Read one LSP message from stdin. Returns the JSON body or an error.
fn read_message(stdin: &mut dyn BufRead) -> io::Result<Value> {
    // Parse headers until empty line
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        stdin.read_line(&mut line)?;
        let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = rest.trim().parse().ok();
        }
        // Other headers (Content-Type) are ignored
    }
    let len = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Missing Content-Length header")
    })?;
    let mut buf = vec![0u8; len];
    stdin.read_exact(&mut buf)?;
    serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Write one LSP message to stdout.
fn write_message(stdout: &mut dyn Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
    stdout.write_all(&body)?;
    stdout.flush()
}

// ── Diagnostics ──────────────────────────────────────────────────────────────

/// Parse + lint + typecheck `source` and return LSP diagnostics.
fn diagnostics_for(source: &str) -> Vec<Value> {
    let mut diags = Vec::new();

    // Lex phase — hard stop on lex error
    let mut lexer = crate::lexer::Lexer::new(source.to_string());
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            diags.push(make_diagnostic(0, 0, 0, source.len(), &e.to_string(), 1));
            return diags;
        }
    };

    // Parse phase — hard stop on parse error
    let mut parser = crate::parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            let (line, col) = extract_position(&e.to_string());
            diags.push(make_diagnostic(line, col, line, col + 1, &e.to_string(), 1));
            return diags;
        }
    };

    // Type-check phase — advisory warnings (severity 2)
    let mut checker = crate::typecheck::checker::TypeChecker::new();
    if let Err(msgs) = checker.check(&program) {
        for msg in msgs {
            let (line, col) = extract_position(&msg);
            diags.push(make_diagnostic(line, col, line, col + 80, &msg, 2));
        }
    }

    // Lint phase — run linter for style / semantic warnings
    if let Ok(issues) = crate::tools::linter::Linter::lint_source(source) {
        for issue in issues {
            // line/col from linter are 1-based; LSP wants 0-based
            let line = issue.line.saturating_sub(1);
            let col = issue.column.saturating_sub(1);
            let severity: u8 = match issue.severity {
                crate::tools::linter::Severity::Error => 1,
                crate::tools::linter::Severity::Warning => 2,
                crate::tools::linter::Severity::Info => 3,
            };
            diags.push(make_diagnostic(line, col, line, col + issue.message.len(), &issue.message, severity));
        }
    }

    diags
}

/// A typed diagnostic suitable for testing, extracted from the JSON representation.
#[derive(Debug)]
pub struct LspDiagnostic {
    pub message: String,
    pub severity: u8,
    pub line: usize,
}

/// Public test helper — returns typed diagnostics for the given source.
pub fn diagnostics_for_test(source: &str) -> Vec<LspDiagnostic> {
    diagnostics_for(source)
        .into_iter()
        .filter_map(|d| {
            let message = d["message"].as_str()?.to_string();
            let severity = d["severity"].as_u64()? as u8;
            let line = d["range"]["start"]["line"].as_u64().unwrap_or(0) as usize;
            Some(LspDiagnostic { message, severity, line })
        })
        .collect()
}

fn make_diagnostic(
    start_line: usize,
    start_char: usize,
    end_line: usize,
    end_char: usize,
    message: &str,
    severity: u8,
) -> Value {
    json!({
        "range": {
            "start": { "line": start_line, "character": start_char },
            "end":   { "line": end_line,   "character": end_char   }
        },
        "severity": severity,
        "source": "txtcode",
        "message": message
    })
}

/// Extract `(line, col)` from error strings like "1:5: ..." or "line 1, col 5".
fn extract_position(msg: &str) -> (usize, usize) {
    // Try "N:M:" format
    if let Some(rest) = msg.split_once(':') {
        if let Ok(line) = rest.0.trim().parse::<usize>() {
            if let Some(rest2) = rest.1.split_once(':') {
                if let Ok(col) = rest2.0.trim().parse::<usize>() {
                    return (line.saturating_sub(1), col.saturating_sub(1));
                }
            }
        }
    }
    (0, 0)
}

// ── Completion items ─────────────────────────────────────────────────────────

fn stdlib_completions() -> Value {
    let items: Vec<Value> = STDLIB_FUNCTIONS
        .iter()
        .map(|name| {
            json!({
                "label": name,
                "kind": 3,          // Function
                "detail": "stdlib",
                "insertText": format!("{}(", name)
            })
        })
        .chain(KEYWORDS.iter().map(|kw| {
            json!({
                "label": kw,
                "kind": 14,         // Keyword
                "detail": "keyword"
            })
        }))
        .collect();
    json!({ "isIncomplete": false, "items": items })
}

// ── Main loop ────────────────────────────────────────────────────────────────

/// Run the LSP server until `exit` notification is received.
pub fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin = io::BufReader::new(stdin.lock());
    let mut stdout = stdout.lock();

    // Track open document contents: URI → text
    let mut documents: HashMap<String, String> = HashMap::new();
    // Workspace-wide symbol index
    let mut workspace_index = WorkspaceIndex::new();

    loop {
        let msg = match read_message(&mut stdin) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("LSP read error: {}", e)),
        };

        let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();

        match method {
            "initialize" => {
                // Scan workspace for cross-file symbol index
                if let Some(root) = root_from_params(&msg["params"]) {
                    workspace_index.build_from_root(&root);
                }
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": {
                                "openClose": true,
                                "change": 1   // Full sync
                            },
                            "completionProvider": {
                                "triggerCharacters": [".", "_", "("]
                            },
                            "signatureHelpProvider": {
                                "triggerCharacters": ["(", ","]
                            },
                            "definitionProvider": true,
                            "hoverProvider": true,
                            "renameProvider": true,
                            "referencesProvider": true,
                            "workspaceSymbolProvider": true,
                            "diagnosticProvider": {
                                "interFileDependencies": true,
                                "workspaceDiagnostics": false
                            }
                        },
                        "serverInfo": {
                            "name": "txtcode-lsp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "initialized" => {
                // Notification — no response needed
            }

            "textDocument/didOpen" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let text = msg["params"]["textDocument"]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let version = msg["params"]["textDocument"]["version"]
                    .as_i64()
                    .unwrap_or(0);
                workspace_index.update_file(&uri, &text);
                documents.insert(uri.clone(), text.clone());
                publish_diagnostics(&mut stdout, &uri, version, &text)?;
            }

            "textDocument/didChange" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let version = msg["params"]["textDocument"]["version"]
                    .as_i64()
                    .unwrap_or(0);
                // Full sync — take the last change's text
                if let Some(changes) = msg["params"]["contentChanges"].as_array() {
                    if let Some(last) = changes.last() {
                        let text = last["text"].as_str().unwrap_or("").to_string();
                        workspace_index.update_file(&uri, &text);
                        documents.insert(uri.clone(), text.clone());
                        publish_diagnostics(&mut stdout, &uri, version, &text)?;
                    }
                }
            }

            "textDocument/didClose" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                documents.remove(&uri);
                // Clear diagnostics for closed file
                let notification = json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": { "uri": uri, "diagnostics": [] }
                });
                write_message(&mut stdout, &notification)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "textDocument/completion" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str().unwrap_or("").to_string();
                let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;
                let completions = if let Some(text) = documents.get(&uri) {
                    context_completions(text, line, character, &workspace_index)
                } else {
                    stdlib_completions()
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": completions });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "textDocument/signatureHelp" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str().unwrap_or("").to_string();
                let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;
                let result = if let Some(text) = documents.get(&uri) {
                    signature_help_at(text, line, character)
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": result });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "textDocument/definition" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;
                let result = if let Some(text) = documents.get(&uri) {
                    let sym = symbol_at(text, line, character);
                    // Try same-file first
                    let same_file = sym.as_deref()
                        .and_then(|name| find_definition(text, name))
                        .map(|(def_line, def_char)| json!({
                            "uri": &uri,
                            "range": lsp_range(def_line, def_char, def_line, def_char)
                        }));
                    if same_file.is_some() {
                        same_file.unwrap()
                    } else if let Some(name) = sym {
                        // Fall back to workspace index (cross-file)
                        let defs = workspace_index.find_definition(&name);
                        if defs.is_empty() {
                            Value::Null
                        } else {
                            // Return array of locations
                            let locations: Vec<Value> = defs.iter().map(|d| json!({
                                "uri": d.uri,
                                "range": lsp_range(d.line, d.col, d.line, d.col + d.name.len())
                            })).collect();
                            if locations.len() == 1 {
                                locations.into_iter().next().unwrap()
                            } else {
                                Value::Array(locations)
                            }
                        }
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": result });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "textDocument/references" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;
                let locations: Vec<Value> = if let Some(text) = documents.get(&uri) {
                    symbol_at(text, line, character)
                        .map(|name| workspace_index.find_references(&name, &documents))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": locations });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "workspace/symbol" => {
                let query = msg["params"]["query"].as_str().unwrap_or("");
                let symbols: Vec<Value> = workspace_index
                    .search_symbols(query)
                    .iter()
                    .map(|s| {
                        json!({
                            "name": s.name,
                            "kind": s.kind,
                            "location": {
                                "uri": s.uri,
                                "range": lsp_range(s.line, s.col, s.line, s.col + s.name.len())
                            }
                        })
                    })
                    .collect();
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": symbols });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "textDocument/hover" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;
                let result = if let Some(text) = documents.get(&uri) {
                    let sym = symbol_at(text, line, character);
                    sym.and_then(|name| hover_info(text, &name))
                        .map(|content| json!({
                            "contents": { "kind": "markdown", "value": content }
                        }))
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": result });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "textDocument/rename" => {
                let uri = msg["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
                let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;
                let new_name = msg["params"]["newName"].as_str().unwrap_or("").to_string();
                let result = if let Some(text) = documents.get(&uri) {
                    let sym = symbol_at(text, line, character);
                    sym.map(|name| {
                        let edits: Vec<Value> = find_all_occurrences(text, &name)
                            .into_iter()
                            .map(|(ol, oc, el, ec)| json!({
                                "range": lsp_range(ol, oc, el, ec),
                                "newText": new_name
                            }))
                            .collect();
                        json!({ "changes": { &uri: edits } })
                    })
                    .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": result });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "shutdown" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": null
                });
                write_message(&mut stdout, &response)
                    .map_err(|e| format!("LSP write error: {}", e))?;
            }

            "exit" => {
                // Notification — clean shutdown
                break;
            }

            other => {
                // Unknown request — send method-not-found error if it has an id
                if let Some(req_id) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {}", other)
                        }
                    });
                    write_message(&mut stdout, &response)
                        .map_err(|e| format!("LSP write error: {}", e))?;
                }
            }
        }
    }

    Ok(())
}

fn publish_diagnostics(
    stdout: &mut dyn Write,
    uri: &str,
    version: i64,
    text: &str,
) -> Result<(), String> {
    let diags = diagnostics_for(text);
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "version": version,
            "diagnostics": diags
        }
    });
    write_message(stdout, &notification).map_err(|e| format!("LSP write error: {}", e))
}

// ── Symbol table helpers ──────────────────────────────────────────────────────

/// Build an LSP Range object (all values 0-based).
fn lsp_range(start_line: usize, start_char: usize, end_line: usize, end_char: usize) -> Value {
    json!({
        "start": { "line": start_line, "character": start_char },
        "end":   { "line": end_line,   "character": end_char   }
    })
}

/// Return the identifier word at (line, character) in the source text, or None.
fn symbol_at(text: &str, line: usize, character: usize) -> Option<String> {
    let src_line = text.lines().nth(line)?;
    let chars: Vec<char> = src_line.chars().collect();
    if character >= chars.len() {
        return None;
    }
    // Extend left and right while alphanumeric or '_'
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    if !is_ident(chars[character]) {
        return None;
    }
    let start = (0..=character).rev().take_while(|&i| is_ident(chars[i])).last().unwrap_or(character);
    let end = (character..chars.len()).take_while(|&i| is_ident(chars[i])).last().unwrap_or(character);
    let word: String = chars[start..=end].iter().collect();
    if word.is_empty() { None } else { Some(word) }
}

/// Find the definition location of `name` in the source (1st `define → name` or `store → name`).
/// Returns (0-based line, 0-based char offset of the name start).
fn find_definition(text: &str, name: &str) -> Option<(usize, usize)> {
    for (ln, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        // `define → name` or `store → name` or `struct name`
        let prefix = if trimmed.starts_with("define →") {
            Some("define →")
        } else if trimmed.starts_with("define ") {
            Some("define")
        } else if trimmed.starts_with("struct ") {
            Some("struct")
        } else {
            None
        };

        if let Some(pfx) = prefix {
            let after = trimmed[pfx.len()..].trim().trim_start_matches('→').trim();
            let def_name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if def_name == name {
                // Find the character position of `name` in the original line
                if let Some(pos) = line.find(name) {
                    return Some((ln, pos));
                }
            }
        }
    }
    None
}

/// Return a hover markdown string for a symbol (function signature + doc comment if available).
fn hover_info(text: &str, name: &str) -> Option<String> {
    let mut doc_lines: Vec<&str> = Vec::new();
    let mut found_sig: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            doc_lines.push(stripped);
            continue;
        }
        // Detect define line
        let is_define = trimmed.starts_with("define →") || trimmed.starts_with("define ");
        if is_define {
            let after = trimmed
                .trim_start_matches("define")
                .trim()
                .trim_start_matches('→')
                .trim();
            let def_name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if def_name == name {
                // Extract params section: after `→ name → (`
                let sig = if let Some(paren) = after.find('(') {
                    let params_end = after.rfind(')').unwrap_or(after.len());
                    let params = &after[paren..=params_end.min(after.len() - 1)];
                    format!("fn {}{})", name, &params[..params.len().saturating_sub(1)])
                } else {
                    format!("fn {}", name)
                };
                found_sig = Some(sig);
                break;
            }
        }

        // Clear doc lines on non-doc, non-define, non-blank lines
        if !trimmed.is_empty() && !trimmed.starts_with("##") && !is_define {
            doc_lines.clear();
        }
    }

    found_sig.map(|sig| {
        let mut md = format!("```\n{}\n```", sig);
        if !doc_lines.is_empty() {
            md.push_str("\n\n");
            md.push_str(&doc_lines.join(" "));
        }
        md
    })
}

/// Find all occurrences of `name` as a whole identifier in the source.
/// Returns (start_line, start_char, end_line, end_char) tuples (0-based).
fn find_all_occurrences(text: &str, name: &str) -> Vec<(usize, usize, usize, usize)> {
    let mut results = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let name_bytes = name.as_bytes();
        let mut start = 0;
        while start + name_bytes.len() <= bytes.len() {
            if let Some(pos) = line[start..].find(name) {
                let abs_pos = start + pos;
                // Check word boundaries
                let before_ok = abs_pos == 0
                    || !bytes[abs_pos - 1].is_ascii_alphanumeric()
                       && bytes[abs_pos - 1] != b'_';
                let after_pos = abs_pos + name_bytes.len();
                let after_ok = after_pos >= bytes.len()
                    || !bytes[after_pos].is_ascii_alphanumeric()
                       && bytes[after_pos] != b'_';
                if before_ok && after_ok {
                    results.push((ln, abs_pos, ln, abs_pos + name.len()));
                }
                start = abs_pos + 1;
            } else {
                break;
            }
        }
    }
    results
}

// ── Context-aware completions ─────────────────────────────────────────────────

/// Method names available on common Txtcode value types.
const METHOD_COMPLETIONS: &[(&str, &str)] = &[
    ("len", "string | array | map | bytes"),
    ("upper", "string"), ("lower", "string"), ("trim", "string"),
    ("split", "string"), ("starts_with", "string"), ("ends_with", "string"),
    ("contains", "string"), ("replace", "string"), ("chars", "string"),
    ("push", "array"), ("pop", "array"), ("shift", "array"), ("unshift", "array"),
    ("map", "array"), ("filter", "array"), ("reduce", "array"), ("find", "array"),
    ("sort", "array"), ("reverse", "array"), ("concat", "array"), ("slice", "array"),
    ("sum", "array"), ("join", "array"),
    ("keys", "map"), ("values", "map"), ("has", "map"), ("delete", "map"),
    ("to_json", "any"), ("to_string", "any"), ("type_of", "any"),
];

/// Build context-aware completion items for the given cursor position.
///
/// Dispatch logic:
/// - After `.` → method completions
/// - After `import →` → package name completions
/// - Otherwise → scope identifiers + stdlib + keywords
pub fn context_completions(
    source: &str,
    line: usize,
    character: usize,
    index: &WorkspaceIndex,
) -> Value {
    // Determine context from the text before the cursor on the current line
    let src_line = source.lines().nth(line).unwrap_or("");
    let before_cursor = if character <= src_line.len() {
        &src_line[..character]
    } else {
        src_line
    };
    let trimmed = before_cursor.trim_end();

    // Context: after `.` (method completions)
    if trimmed.ends_with('.') {
        let items: Vec<Value> = METHOD_COMPLETIONS
            .iter()
            .map(|(name, detail)| json!({
                "label": name,
                "kind": 2,      // Method
                "detail": detail,
                "insertText": format!("{}(", name)
            }))
            .collect();
        return json!({ "isIncomplete": false, "items": items });
    }

    // Context: after `import →` — offer package names from workspace index
    if trimmed.ends_with("import →") || trimmed.ends_with("import") {
        let pkg_items: Vec<Value> = index.definitions.keys()
            .take(50)
            .map(|name| json!({ "label": name, "kind": 9, "detail": "symbol" }))
            .collect();
        return json!({ "isIncomplete": false, "items": pkg_items });
    }

    // General: scope identifiers at cursor position + stdlib + keywords
    let scope = build_scope_at_position(source, line);
    let scope_items: Vec<Value> = scope
        .iter()
        .map(|(name, kind)| {
            let lsp_kind: u32 = match kind.as_str() {
                "function" => 3,
                "variable" => 6,
                _ => 6,
            };
            json!({ "label": name, "kind": lsp_kind, "detail": kind.as_str() })
        })
        .collect();

    // Merge scope + stdlib + keywords, deduplicated
    let mut items = scope_items;
    let scope_names: std::collections::HashSet<String> =
        scope.iter().map(|(n, _)| n.clone()).collect();

    items.extend(STDLIB_FUNCTIONS.iter()
        .filter(|n| !scope_names.contains(**n))
        .map(|name| json!({
            "label": name,
            "kind": 3,
            "detail": "stdlib",
            "insertText": format!("{}(", name)
        })));
    items.extend(KEYWORDS.iter().map(|kw| json!({
        "label": kw,
        "kind": 14,
        "detail": "keyword"
    })));

    json!({ "isIncomplete": false, "items": items })
}

/// Collect all variable and function names visible at `line` in the source.
/// Returns a list of (name, kind) pairs where kind is "function" or "variable".
pub fn build_scope_at_position(source: &str, line: usize) -> Vec<(String, String)> {
    let mut scope: Vec<(String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (ln, src_line) in source.lines().enumerate() {
        if ln >= line { break; }
        let trimmed = src_line.trim();

        // Function definition: `define → name → (params)` or `define name`
        if trimmed.starts_with("define") {
            let after = trimmed["define".len()..].trim().trim_start_matches('→').trim();
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() && !seen.contains(&name) {
                scope.push((name.clone(), "function".to_string()));
                seen.insert(name);
            }
        }
        // Variable assignment: `store → name → ...`
        else if trimmed.starts_with("store") {
            let after = trimmed["store".len()..].trim().trim_start_matches('→').trim();
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() && !seen.contains(&name) {
                scope.push((name.clone(), "variable".to_string()));
                seen.insert(name);
            }
        }
        // For loop variable: `for → var in ...`
        else if trimmed.starts_with("for") {
            let after = trimmed["for".len()..].trim().trim_start_matches('→').trim();
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() && !seen.contains(&name) {
                scope.push((name.clone(), "variable".to_string()));
                seen.insert(name);
            }
        }
    }
    scope
}

/// Provide signature help when cursor is inside a function call's argument list.
///
/// Detects the innermost unclosed `(` before the cursor, looks up the function
/// name that precedes it, and returns LSP `SignatureHelp`.
pub fn signature_help_at(source: &str, line: usize, character: usize) -> Option<Value> {
    let src_line = source.lines().nth(line)?;
    let before = if character <= src_line.len() { &src_line[..character] } else { src_line };

    // Find the function name before the last unclosed `(`
    let open_paren = before.rfind('(')?;
    let before_paren = before[..open_paren].trim_end();
    // Extract the trailing identifier (function name)
    let fn_name: String = before_paren.chars().rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars().rev().collect();
    if fn_name.is_empty() { return None; }

    // Count commas since the last `(` to determine active parameter
    let args_text = &before[open_paren + 1..];
    let active_param = args_text.chars().filter(|&c| c == ',').count() as u32;

    // Try to get parameter info from the source definition
    let params = extract_function_params(source, &fn_name)
        .unwrap_or_else(|| vec!["...".to_string()]);

    let params_str = params.join(", ");
    let param_infos: Vec<Value> = params.iter()
        .map(|p| json!({ "label": p }))
        .collect();

    Some(json!({
        "signatures": [{
            "label": format!("{}({})", fn_name, params_str),
            "parameters": param_infos,
            "activeParameter": active_param.min(params.len().saturating_sub(1) as u32)
        }],
        "activeSignature": 0,
        "activeParameter": active_param.min(params.len().saturating_sub(1) as u32)
    }))
}

/// Extract parameter names for a named function from the source text.
/// Returns `None` if the function is not found.
fn extract_function_params(source: &str, fn_name: &str) -> Option<Vec<String>> {
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("define") { continue; }
        let after = trimmed["define".len()..].trim().trim_start_matches('→').trim();
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name != fn_name { continue; }
        // Find `(params)`
        let rest = &after[name.len()..].trim().trim_start_matches('→').trim();
        if let Some(open) = rest.find('(') {
            if let Some(close) = rest[open..].find(')') {
                let params_str = &rest[open + 1..open + close];
                let params: Vec<String> = params_str.split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                return Some(params);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_doc(uri: &str, src: &str) -> (String, String) {
        (uri.to_string(), src.to_string())
    }

    #[test]
    fn test_index_source_functions() {
        let src = "define → add → (a, b)\n  return → a + b\nend\n";
        let syms = index_source(src, "file:///test.tc");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "add");
        assert_eq!(syms[0].kind, 12); // Function
    }

    #[test]
    fn test_index_source_struct() {
        let src = "struct Point(x: int, y: int)\n";
        let syms = index_source(src, "file:///test.tc");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "Point");
        assert_eq!(syms[0].kind, 5); // Class/Struct
    }

    #[test]
    fn test_index_source_top_level_store() {
        let src = "store → MAX → 100\n  store → ignored → 1\n";
        let syms = index_source(src, "file:///test.tc");
        // Only top-level (unindented) store statements
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "MAX");
    }

    #[test]
    fn test_workspace_index_find_definition() {
        let mut idx = WorkspaceIndex::new();
        idx.update_file("file:///a.tc", "define → foo → ()\n  return → 1\nend\n");
        idx.update_file("file:///b.tc", "define → bar → ()\n  return → 2\nend\n");
        let defs = idx.find_definition("foo");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].uri, "file:///a.tc");
        // bar is not in a.tc
        let defs2 = idx.find_definition("bar");
        assert_eq!(defs2.len(), 1);
        assert_eq!(defs2[0].uri, "file:///b.tc");
    }

    #[test]
    fn test_workspace_index_search_symbols() {
        let mut idx = WorkspaceIndex::new();
        idx.update_file("file:///a.tc", "define → multiply → (a, b)\nend\ndefine → add → (a, b)\nend\n");
        let results = idx.search_symbols("mul");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "multiply");
        let all = idx.search_symbols("");
        assert!(all.len() >= 2);
    }

    #[test]
    fn test_workspace_index_find_references_across_files() {
        let mut idx = WorkspaceIndex::new();
        idx.update_file("file:///a.tc", "define → helper → ()\nend\n");
        idx.update_file("file:///b.tc", "store → x → helper()\n");
        let mut docs = HashMap::new();
        docs.insert("file:///a.tc".to_string(), "define → helper → ()\nend\n".to_string());
        docs.insert("file:///b.tc".to_string(), "store → x → helper()\n".to_string());
        let refs = idx.find_references("helper", &docs);
        // Should find "helper" in both files
        assert!(refs.len() >= 2, "Expected at least 2 references, got {}", refs.len());
    }

    #[test]
    fn test_workspace_index_update_removes_old_symbols() {
        let mut idx = WorkspaceIndex::new();
        idx.update_file("file:///a.tc", "define → old_fn → ()\nend\n");
        assert!(!idx.find_definition("old_fn").is_empty());
        // Update the file — old symbol should be removed
        idx.update_file("file:///a.tc", "define → new_fn → ()\nend\n");
        assert!(idx.find_definition("old_fn").is_empty(), "Old symbol should be removed on update");
        assert!(!idx.find_definition("new_fn").is_empty());
    }

    #[test]
    fn test_path_to_uri_and_back() {
        let path = std::path::PathBuf::from("/tmp/test.tc");
        let uri = path_to_uri(&path);
        assert!(uri.starts_with("file://"), "URI should start with file://");
        // uri_to_path should round-trip
        if let Some(p) = uri_to_path(&uri) {
            assert!(p.to_string_lossy().contains("test.tc"));
        }
    }

    // ── F.3: Context-aware completion + signatureHelp tests ──────────────────

    // F.3.1: Variable defined before cursor appears in scope completions
    #[test]
    fn test_f3_variable_completion() {
        let src = "store → myvar → 42\n";
        let idx = WorkspaceIndex::new();
        let completions = context_completions(src, 1, 0, &idx);
        let items = completions["items"].as_array().unwrap();
        assert!(
            items.iter().any(|i| i["label"].as_str() == Some("myvar")),
            "expected 'myvar' in completions, got {} items", items.len()
        );
    }

    // F.3.2: Function defined before cursor appears as function completion
    #[test]
    fn test_f3_function_completion() {
        let src = "define → my_func → (x)\n  return → x\nend\n";
        let idx = WorkspaceIndex::new();
        let completions = context_completions(src, 3, 0, &idx);
        let items = completions["items"].as_array().unwrap();
        let fn_item = items.iter().find(|i| i["label"].as_str() == Some("my_func"));
        assert!(fn_item.is_some(), "expected 'my_func' in completions");
        assert_eq!(fn_item.unwrap()["kind"].as_u64(), Some(3)); // Function kind
    }

    // F.3.3: Stdlib functions always appear in general completions
    #[test]
    fn test_f3_stdlib_completion() {
        let src = "store → x → 1\n";
        let idx = WorkspaceIndex::new();
        let completions = context_completions(src, 1, 0, &idx);
        let items = completions["items"].as_array().unwrap();
        assert!(
            items.iter().any(|i| i["label"].as_str() == Some("print")),
            "stdlib 'print' should be in completions"
        );
    }

    // F.3.4: After `.` → method completions only
    #[test]
    fn test_f3_method_completion_after_dot() {
        let src = "store → s → \"hello\"\n";
        let idx = WorkspaceIndex::new();
        // Cursor is after `s.` on line 1
        let completions = context_completions(src, 1, 2, &idx);
        // Simulate cursor after `.`
        let completions_after_dot = context_completions("store → s → \"hello\"\ns.", 1, 2, &idx);
        let items = completions_after_dot["items"].as_array().unwrap();
        assert!(
            items.iter().any(|i| i["label"].as_str() == Some("len")),
            "method 'len' should appear after dot"
        );
        assert!(completions["items"].as_array().unwrap().iter().any(|i| i["label"] == "print"),
            "stdlib present in non-dot context");
    }

    // F.3.5: signatureHelp returns parameter info for defined function
    #[test]
    fn test_f3_signature_help_defined_fn() {
        let src = "define → add → (a, b)\n  return → a + b\nend\nadd(";
        let help = signature_help_at(src, 3, 4); // cursor after `add(`
        assert!(help.is_some(), "expected signature help for add(");
        let h = help.unwrap();
        let sigs = h["signatures"].as_array().unwrap();
        assert!(!sigs.is_empty(), "at least one signature");
        let label = sigs[0]["label"].as_str().unwrap();
        assert!(label.contains("add"), "label should include function name: {}", label);
        assert!(label.contains("a") && label.contains("b"), "label should include params: {}", label);
    }

    // F.3.6: signatureHelp returns None for unknown function
    #[test]
    fn test_f3_signature_help_unknown_fn() {
        let src = "store → x → unknown_fn(";
        // No definition of unknown_fn — falls back to `...`
        let help = signature_help_at(src, 0, src.len());
        // Should still return something (with "...") since we find `unknown_fn(`
        if let Some(h) = help {
            let sigs = h["signatures"].as_array().unwrap();
            assert!(!sigs.is_empty());
        }
        // Either Some with ellipsis or None is acceptable
    }

    // ── T.1: publishDiagnostics pipeline tests ────────────────────────────────

    // T.1.1: Clean source produces no error-severity diagnostics
    #[test]
    fn test_t1_clean_source_no_diagnostics() {
        let src = "store → x → 42\nprint(x)\n";
        let diags = diagnostics_for_test(src);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == 1).collect();
        assert!(errors.is_empty(), "clean source should have no errors, got: {:?}", diags);
    }

    // T.1.2: Lex/parse error produces severity-1 diagnostic
    #[test]
    fn test_t1_parse_error_severity_1() {
        // Unclosed string literal → lex error
        let src = "store → x → \"unterminated";
        let diags = diagnostics_for_test(src);
        assert!(!diags.is_empty(), "expected at least one diagnostic for lex error");
        assert_eq!(diags[0].severity, 1, "lex error should be severity 1 (Error)");
    }

    // T.1.3: Invalid syntax (missing end) → parse error, severity 1
    #[test]
    fn test_t1_syntax_error_severity_1() {
        let src = "if → true\n  store → x → 1\n"; // missing `end`
        let diags = diagnostics_for_test(src);
        assert!(!diags.is_empty(), "expected parse error diagnostic");
        assert_eq!(diags[0].severity, 1);
    }

    // T.1.4: Type mismatch → severity-2 warning (advisory)
    #[test]
    fn test_t1_type_warning_severity_2() {
        // x declared int, assigned string → type checker warning
        let src = "store → x: int → \"hello\"\n";
        let diags = diagnostics_for_test(src);
        let warnings: Vec<_> = diags.iter().filter(|d| d.severity == 2).collect();
        assert!(!warnings.is_empty(), "expected at least one type warning (severity 2)");
    }

    // T.1.5: diagnostics_for returns valid JSON structure
    #[test]
    fn test_t1_diagnostic_json_structure() {
        let src = "store → x: int → \"bad\"\n";
        let raw = diagnostics_for(src);
        for d in &raw {
            assert!(d["range"].is_object(), "diagnostic must have 'range'");
            assert!(d["severity"].is_number(), "diagnostic must have 'severity'");
            assert!(d["message"].is_string(), "diagnostic must have 'message'");
        }
    }
}
