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

use serde_json::{json, Value};

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

/// Parse `source` and return LSP diagnostics (errors only).
fn diagnostics_for(source: &str) -> Vec<Value> {
    let mut diags = Vec::new();

    // Lex phase
    let mut lexer = crate::lexer::Lexer::new(source.to_string());
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            diags.push(make_diagnostic(0, 0, 0, source.len(), &e.to_string(), 1));
            return diags;
        }
    };

    // Parse phase
    let mut parser = crate::parser::Parser::new(tokens);
    if let Err(e) = parser.parse() {
        // Try to extract line/col from error message (format: "line X col Y: ...")
        let (line, col) = extract_position(&e.to_string());
        diags.push(make_diagnostic(line, col, line, col + 1, &e.to_string(), 1));
    }

    diags
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
                                "triggerCharacters": ["_", "("]
                            },
                            "definitionProvider": true,
                            "hoverProvider": true,
                            "renameProvider": true,
                            "diagnosticProvider": {
                                "interFileDependencies": false,
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
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": stdlib_completions()
                });
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
                    sym.and_then(|name| find_definition(text, &name))
                        .map(|(def_line, def_char)| json!({
                            "uri": uri,
                            "range": lsp_range(def_line, def_char, def_line, def_char)
                        }))
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": result });
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
