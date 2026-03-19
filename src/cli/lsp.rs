//! Minimal Language Server Protocol (LSP) server for Txtcode.
//!
//! Communicates over stdin/stdout using the JSON-RPC 2.0 framing mandated by LSP.
//! No external LSP crate required — only `serde_json` (already a dependency).
//!
//! Supported capabilities:
//! - `initialize` / `initialized`
//! - `textDocument/didOpen`, `textDocument/didChange` → `publishDiagnostics`
//! - `textDocument/completion` → stdlib + keyword completions
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
