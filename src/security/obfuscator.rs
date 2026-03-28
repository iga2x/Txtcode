// AST-level identifier obfuscation — two-pass name-mangling implementation.
//
// Pass 1: collect user-defined variable/function/parameter names.
// Pass 2: substitute all occurrences with generated names `_o0`, `_o1`, ...
//
// Names that are NOT mangled:
//   - Stdlib function names (checked against STDLIB_NAMES)
//   - Names starting with `__` (internal runtime names)
//   - Struct field names and map literal keys (string literals)
//   - Type alias names and named error names

use crate::parser::ast::{
    Expression, InterpolatedSegment, Pattern, Program, Statement,
};
use std::collections::HashMap;

/// Known stdlib function names that must not be mangled.
const STDLIB_NAMES: &[&str] = &[
    // core
    "print", "len", "type", "string", "int", "float", "bool", "input",
    "max", "min", "map", "filter", "reduce", "find", "sort", "reverse",
    "concat", "split", "join", "replace", "trim", "substring", "indexOf",
    "startsWith", "endsWith", "toUpper", "toLower", "set",
    "ok", "err", "is_ok", "is_err", "unwrap", "unwrap_or",
    "base32_encode", "base32_decode", "html_escape", "xml_decode", "xml_parse",
    "math_random_float",
    // math
    "sin", "cos", "tan", "sqrt", "log", "pow", "abs", "floor", "ceil", "round",
    // crypto
    "sha256", "sha512", "sha1", "md5", "encrypt", "decrypt",
    "base64_encode", "base64_decode", "hmac_sha256", "uuid_v4",
    "secure_compare", "pbkdf2", "bcrypt_hash", "bcrypt_verify",
    "ed25519_sign", "ed25519_verify", "rsa_generate", "rsa_sign", "rsa_verify",
    "crypto_random_int", "crypto_random_bytes", "crypto_random_float",
    // net
    "http_get", "http_post", "http_put", "http_delete", "http_patch",
    "http_head", "http_options", "http_request",
    "tcp_connect", "tcp_listen", "tcp_accept", "tcp_send", "tcp_recv",
    "udp_send", "resolve",
    // io/fs
    "read_file", "write_file", "append_file", "delete", "file_exists",
    "is_file", "is_dir", "mkdir", "rmdir", "list_dir", "copy_file",
    "move_file", "rename_file", "temp_file", "watch_file",
    "read_lines", "symlink_create", "zip_create", "zip_extract",
    // sys
    "getenv", "setenv", "platform", "arch",
    "exec", "exec_status", "exec_lines", "exec_json", "exit", "args",
    "cwd", "chdir", "env_list", "signal_send", "pipe_exec", "which",
    "is_root", "cpu_count", "memory_available", "disk_space", "os_name",
    "os_version", "pid", "user", "home", "uid", "gid", "spawn", "kill",
    "wait", "sleep",
    // time
    "now", "format_time", "time_format", "parse_time", "time_parse",
    // json — canonical names; legacy aliases kept for backward compat
    "json_encode", "json_decode", "json_parse", "json_stringify", "json_format",
    // regex
    "regex_match", "regex_find_all", "regex_replace", "regex_split",
    // path
    "path_join", "path_dirname", "path_basename", "path_extension",
    "path_absolute", "path_exists", "path_is_absolute",
    // log — canonical names only (bare debug/info/warn/error removed in v0.4.1)
    "log", "log_debug", "log_info", "log_warn", "log_error",
    // url
    "url_parse", "url_build", "encode_uri", "decode_uri",
    "encode_uri_component", "decode_uri_component",
    // tools
    "tool_exec", "tool_list", "tool_info",
    // ffi
    "ffi_load", "ffi_call", "ffi_close",
    // capabilities
    "grant_capability", "use_capability", "clear_capability", "revoke_capability",
    // str_format alias
    "format",
    // to_ conversions
    "to_string", "to_int", "to_float", "to_bool", "to_array",
    // str_ / math_ / array_ / set_ prefixed — handled by prefix check in should_mangle
];

/// Prefixes that indicate a stdlib function (not mangleable).
const STDLIB_PREFIXES: &[&str] = &[
    "str_", "math_", "array_", "set_", "to_", "json_", "regex_", "path_",
    "log_", "url_", "http", "tcp", "csv_", "toml_", "yaml_", "assert",
    "test_", "ffi_",
];

fn is_stdlib(name: &str) -> bool {
    if STDLIB_NAMES.contains(&name) {
        return true;
    }
    for prefix in STDLIB_PREFIXES {
        if name.starts_with(prefix) {
            return true;
        }
    }
    false
}

fn should_mangle(name: &str) -> bool {
    if name.starts_with("__") {
        return false;
    }
    if is_stdlib(name) {
        return false;
    }
    true
}

/// AST-level code obfuscator implementing identifier mangling.
pub struct Obfuscator {
    /// Map from original name to mangled name.
    name_map: HashMap<String, String>,
    /// Monotonic counter for unique mangled names.
    counter: usize,
}

impl Obfuscator {
    pub fn new() -> Self {
        Self {
            name_map: HashMap::new(),
            counter: 0,
        }
    }

    /// Mangle a name: look up existing mapping or create a new one.
    fn mangle(&mut self, name: &str) -> String {
        if let Some(m) = self.name_map.get(name) {
            return m.clone();
        }
        let mangled = format!("_o{}", self.counter);
        self.counter += 1;
        self.name_map.insert(name.to_string(), mangled.clone());
        mangled
    }

    /// Register a name for mangling without returning the mangled form.
    fn register(&mut self, name: &str) {
        if should_mangle(name) {
            self.mangle(name);
        }
    }

    /// Obfuscate a program with two-pass identifier mangling.
    pub fn obfuscate(&mut self, program: &Program) -> Program {
        // Pass 1: collect all user-defined names
        for stmt in &program.statements {
            self.collect_stmt(stmt);
        }
        // Pass 2: substitute
        Program {
            statements: program.statements.iter().map(|s| self.sub_stmt(s)).collect(),
        }
    }

    // ── Pass 1: collect ──────────────────────────────────────────────────────

    fn collect_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assignment { pattern, value, .. } => {
                self.collect_pattern(pattern);
                self.collect_expr(value);
            }
            Statement::IndexAssignment { target, index, value, .. } => {
                self.collect_expr(target);
                self.collect_expr(index);
                self.collect_expr(value);
            }
            Statement::CompoundAssignment { name, value, .. } => {
                self.register(name);
                self.collect_expr(value);
            }
            Statement::FunctionDef { name, params, body, .. } => {
                self.register(name);
                for param in params {
                    self.register(&param.name);
                }
                for s in body {
                    self.collect_stmt(s);
                }
            }
            Statement::Return { value: Some(v), .. } => { self.collect_expr(v); }
            Statement::Yield { value, .. } => { self.collect_expr(value); }
            Statement::Nursery { body, .. } => {
                for s in body { self.collect_stmt(s); }
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
                self.collect_expr(condition);
                for s in then_branch { self.collect_stmt(s); }
                for (cond, body) in else_if_branches {
                    self.collect_expr(cond);
                    for s in body { self.collect_stmt(s); }
                }
                if let Some(branch) = else_branch {
                    for s in branch { self.collect_stmt(s); }
                }
            }
            Statement::While { condition, body, .. } => {
                self.collect_expr(condition);
                for s in body { self.collect_stmt(s); }
            }
            Statement::DoWhile { body, condition, .. } => {
                for s in body { self.collect_stmt(s); }
                self.collect_expr(condition);
            }
            Statement::For { variable, iterable, body, .. } => {
                self.register(variable);
                self.collect_expr(iterable);
                for s in body { self.collect_stmt(s); }
            }
            Statement::Repeat { count, body, .. } => {
                self.collect_expr(count);
                for s in body { self.collect_stmt(s); }
            }
            Statement::Expression(expr) => self.collect_expr(expr),
            Statement::Assert { condition, message, .. } => {
                self.collect_expr(condition);
                if let Some(msg) = message { self.collect_expr(msg); }
            }
            Statement::Match { value, cases, default, .. } => {
                self.collect_expr(value);
                for (pat, guard, body) in cases {
                    self.collect_pattern(pat);
                    if let Some(g) = guard { self.collect_expr(g); }
                    for s in body { self.collect_stmt(s); }
                }
                if let Some(d) = default {
                    for s in d { self.collect_stmt(s); }
                }
            }
            Statement::Try { body, catch, finally, .. } => {
                for s in body { self.collect_stmt(s); }
                if let Some((var, stmts)) = catch {
                    self.register(var);
                    for s in stmts { self.collect_stmt(s); }
                }
                if let Some(stmts) = finally {
                    for s in stmts { self.collect_stmt(s); }
                }
            }
            Statement::Const { name, value, .. } => {
                self.register(name);
                self.collect_expr(value);
            }
            // Not collecting: TypeAlias, NamedError, Enum, Struct, Permission, Import, Export, Break, Continue
            _ => {}
        }
    }

    fn collect_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(name) => self.register(name),
            Pattern::Array(pats) => {
                for p in pats { self.collect_pattern(p); }
            }
            Pattern::Struct { fields, rest } => {
                for (_, p) in fields { self.collect_pattern(p); }
                if let Some(r) = rest { self.register(r); }
            }
            Pattern::Constructor { args, .. } => {
                for p in args { self.collect_pattern(p); }
            }
            Pattern::Rest(name) => self.register(name),
            _ => {}
        }
    }

    fn collect_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Lambda { params, body, .. } => {
                for p in params { self.register(&p.name); }
                self.collect_expr(body);
            }
            Expression::BinaryOp { left, right, .. } => {
                self.collect_expr(left);
                self.collect_expr(right);
            }
            Expression::UnaryOp { operand, .. } => self.collect_expr(operand),
            Expression::FunctionCall { arguments, .. } => {
                for a in arguments { self.collect_expr(a); }
            }
            Expression::Array { elements, .. } => {
                for e in elements { self.collect_expr(e); }
            }
            Expression::Map { entries, .. } => {
                for (_, v) in entries { self.collect_expr(v); }
            }
            Expression::Set { elements, .. } => {
                for e in elements { self.collect_expr(e); }
            }
            Expression::Index { target, index, .. } => {
                self.collect_expr(target);
                self.collect_expr(index);
            }
            Expression::Member { target, .. } => self.collect_expr(target),
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                self.collect_expr(condition);
                self.collect_expr(true_expr);
                self.collect_expr(false_expr);
            }
            Expression::Slice { target, start, end, step, .. } => {
                self.collect_expr(target);
                if let Some(s) = start { self.collect_expr(s); }
                if let Some(e) = end { self.collect_expr(e); }
                if let Some(st) = step { self.collect_expr(st); }
            }
            Expression::Await { expression, .. } => self.collect_expr(expression),
            Expression::OptionalMember { target, .. } => self.collect_expr(target),
            Expression::OptionalCall { target, arguments, .. } => {
                self.collect_expr(target);
                for a in arguments { self.collect_expr(a); }
            }
            Expression::OptionalIndex { target, index, .. } => {
                self.collect_expr(target);
                self.collect_expr(index);
            }
            Expression::MethodCall { object, arguments, .. } => {
                self.collect_expr(object);
                for a in arguments { self.collect_expr(a); }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, v) in fields { self.collect_expr(v); }
            }
            Expression::Spread { value, .. } => self.collect_expr(value),
            Expression::InterpolatedString { segments, .. } => {
                for seg in segments {
                    if let InterpolatedSegment::Expression(e) = seg {
                        self.collect_expr(e);
                    }
                }
            }
            _ => {}
        }
    }

    // ── Pass 2: substitute ───────────────────────────────────────────────────

    fn sub_stmt(&mut self, stmt: &Statement) -> Statement {
        match stmt {
            Statement::Assignment { pattern, type_annotation, value, span } => {
                Statement::Assignment {
                    pattern: self.sub_pattern(pattern),
                    type_annotation: type_annotation.clone(),
                    value: self.sub_expr(value),
                    span: span.clone(),
                }
            }
            Statement::IndexAssignment { target, index, value, span } => {
                Statement::IndexAssignment {
                    target: self.sub_expr(target),
                    index: self.sub_expr(index),
                    value: self.sub_expr(value),
                    span: span.clone(),
                }
            }
            Statement::CompoundAssignment { name, op, value, span } => {
                Statement::CompoundAssignment {
                    name: self.sub_name(name),
                    op: *op,
                    value: self.sub_expr(value),
                    span: span.clone(),
                }
            }
            Statement::FunctionDef { name, type_params, params, return_type, body, is_async, intent, ai_hint, allowed_actions, forbidden_actions, span } => {
                let new_name = self.sub_name(name);
                let new_params = params.iter().map(|p| {
                    let mut new_p = p.clone();
                    if should_mangle(&p.name) {
                        new_p.name = self.sub_name(&p.name);
                    }
                    new_p
                }).collect();
                let new_body = body.iter().map(|s| self.sub_stmt(s)).collect();
                Statement::FunctionDef {
                    name: new_name,
                    type_params: type_params.clone(),
                    params: new_params,
                    return_type: return_type.clone(),
                    body: new_body,
                    is_async: *is_async,
                    intent: intent.clone(),
                    ai_hint: ai_hint.clone(),
                    allowed_actions: allowed_actions.clone(),
                    forbidden_actions: forbidden_actions.clone(),
                    span: span.clone(),
                }
            }
            Statement::Return { value, span } => {
                Statement::Return {
                    value: value.as_ref().map(|v| self.sub_expr(v)),
                    span: span.clone(),
                }
            }
            Statement::Break { span } => Statement::Break { span: span.clone() },
            Statement::Continue { span } => Statement::Continue { span: span.clone() },
            Statement::Yield { value, span } => Statement::Yield { value: self.sub_expr(value), span: span.clone() },
            Statement::Nursery { body, span } => Statement::Nursery {
                body: body.iter().map(|s| self.sub_stmt(s)).collect(),
                span: span.clone(),
            },
            Statement::If { condition, then_branch, else_if_branches, else_branch, span } => {
                Statement::If {
                    condition: self.sub_expr(condition),
                    then_branch: then_branch.iter().map(|s| self.sub_stmt(s)).collect(),
                    else_if_branches: else_if_branches.iter().map(|(c, b)| {
                        (self.sub_expr(c), b.iter().map(|s| self.sub_stmt(s)).collect())
                    }).collect(),
                    else_branch: else_branch.as_ref().map(|b| b.iter().map(|s| self.sub_stmt(s)).collect()),
                    span: span.clone(),
                }
            }
            Statement::While { condition, body, span } => {
                Statement::While {
                    condition: self.sub_expr(condition),
                    body: body.iter().map(|s| self.sub_stmt(s)).collect(),
                    span: span.clone(),
                }
            }
            Statement::DoWhile { body, condition, span } => {
                Statement::DoWhile {
                    body: body.iter().map(|s| self.sub_stmt(s)).collect(),
                    condition: self.sub_expr(condition),
                    span: span.clone(),
                }
            }
            Statement::For { variable, iterable, body, span } => {
                Statement::For {
                    variable: self.sub_name(variable),
                    iterable: self.sub_expr(iterable),
                    body: body.iter().map(|s| self.sub_stmt(s)).collect(),
                    span: span.clone(),
                }
            }
            Statement::Repeat { count, body, span } => {
                Statement::Repeat {
                    count: self.sub_expr(count),
                    body: body.iter().map(|s| self.sub_stmt(s)).collect(),
                    span: span.clone(),
                }
            }
            Statement::Expression(expr) => Statement::Expression(self.sub_expr(expr)),
            Statement::Assert { condition, message, span } => {
                Statement::Assert {
                    condition: self.sub_expr(condition),
                    message: message.as_ref().map(|m| self.sub_expr(m)),
                    span: span.clone(),
                }
            }
            Statement::Match { value, cases, default, span } => {
                Statement::Match {
                    value: self.sub_expr(value),
                    cases: cases.iter().map(|(pat, guard, body)| {
                        (
                            self.sub_pattern(pat),
                            guard.as_ref().map(|g| self.sub_expr(g)),
                            body.iter().map(|s| self.sub_stmt(s)).collect(),
                        )
                    }).collect(),
                    default: default.as_ref().map(|d| d.iter().map(|s| self.sub_stmt(s)).collect()),
                    span: span.clone(),
                }
            }
            Statement::Try { body, catch, finally, span } => {
                Statement::Try {
                    body: body.iter().map(|s| self.sub_stmt(s)).collect(),
                    catch: catch.as_ref().map(|(var, stmts)| {
                        (self.sub_name(var), stmts.iter().map(|s| self.sub_stmt(s)).collect())
                    }),
                    finally: finally.as_ref().map(|stmts| stmts.iter().map(|s| self.sub_stmt(s)).collect()),
                    span: span.clone(),
                }
            }
            Statement::Const { name, value, span } => {
                Statement::Const {
                    name: self.sub_name(name),
                    value: self.sub_expr(value),
                    span: span.clone(),
                }
            }
            // Pass-through for structural statements (no identifiers to mangle)
            other => other.clone(),
        }
    }

    fn sub_pattern(&mut self, pattern: &Pattern) -> Pattern {
        match pattern {
            Pattern::Identifier(name) => Pattern::Identifier(self.sub_name(name)),
            Pattern::Array(pats) => Pattern::Array(pats.iter().map(|p| self.sub_pattern(p)).collect()),
            Pattern::Struct { fields, rest } => Pattern::Struct {
                fields: fields.iter().map(|(k, p)| (k.clone(), self.sub_pattern(p))).collect(),
                rest: rest.as_ref().map(|r| self.sub_name(r)),
            },
            Pattern::Constructor { type_name, args } => Pattern::Constructor {
                type_name: type_name.clone(), // struct name, not a variable
                args: args.iter().map(|p| self.sub_pattern(p)).collect(),
            },
            Pattern::Rest(name) => Pattern::Rest(self.sub_name(name)),
            other => other.clone(),
        }
    }

    fn sub_name(&mut self, name: &str) -> String {
        if let Some(mangled) = self.name_map.get(name) {
            mangled.clone()
        } else {
            name.to_string()
        }
    }

    fn sub_expr(&mut self, expr: &Expression) -> Expression {
        match expr {
            Expression::Identifier(name) => {
                Expression::Identifier(self.sub_name(name))
            }
            Expression::FunctionCall { name, type_arguments, arguments, span } => {
                Expression::FunctionCall {
                    name: self.sub_name(name),
                    type_arguments: type_arguments.clone(),
                    arguments: arguments.iter().map(|a| self.sub_expr(a)).collect(),
                    span: span.clone(),
                }
            }
            Expression::Lambda { params, body, span } => {
                let new_params = params.iter().map(|p| {
                    let mut new_p = p.clone();
                    if should_mangle(&p.name) {
                        new_p.name = self.sub_name(&p.name);
                    }
                    new_p
                }).collect();
                Expression::Lambda {
                    params: new_params,
                    body: Box::new(self.sub_expr(body)),
                    span: span.clone(),
                }
            }
            Expression::BinaryOp { left, op, right, span } => {
                Expression::BinaryOp {
                    left: Box::new(self.sub_expr(left)),
                    op: *op,
                    right: Box::new(self.sub_expr(right)),
                    span: span.clone(),
                }
            }
            Expression::UnaryOp { op, operand, span } => {
                Expression::UnaryOp {
                    op: *op,
                    operand: Box::new(self.sub_expr(operand)),
                    span: span.clone(),
                }
            }
            Expression::Array { elements, span } => {
                Expression::Array {
                    elements: elements.iter().map(|e| self.sub_expr(e)).collect(),
                    span: span.clone(),
                }
            }
            Expression::Map { entries, span } => {
                Expression::Map {
                    // Keys (string literals) are NOT mangled; only values
                    entries: entries.iter().map(|(k, v)| (k.clone(), self.sub_expr(v))).collect(),
                    span: span.clone(),
                }
            }
            Expression::Set { elements, span } => {
                Expression::Set {
                    elements: elements.iter().map(|e| self.sub_expr(e)).collect(),
                    span: span.clone(),
                }
            }
            Expression::Index { target, index, span } => {
                Expression::Index {
                    target: Box::new(self.sub_expr(target)),
                    index: Box::new(self.sub_expr(index)),
                    span: span.clone(),
                }
            }
            Expression::Member { target, name, span } => {
                Expression::Member {
                    target: Box::new(self.sub_expr(target)),
                    name: name.clone(), // field name, not a variable
                    span: span.clone(),
                }
            }
            Expression::Ternary { condition, true_expr, false_expr, span } => {
                Expression::Ternary {
                    condition: Box::new(self.sub_expr(condition)),
                    true_expr: Box::new(self.sub_expr(true_expr)),
                    false_expr: Box::new(self.sub_expr(false_expr)),
                    span: span.clone(),
                }
            }
            Expression::Slice { target, start, end, step, span } => {
                Expression::Slice {
                    target: Box::new(self.sub_expr(target)),
                    start: start.as_ref().map(|s| Box::new(self.sub_expr(s))),
                    end: end.as_ref().map(|e| Box::new(self.sub_expr(e))),
                    step: step.as_ref().map(|s| Box::new(self.sub_expr(s))),
                    span: span.clone(),
                }
            }
            Expression::Await { expression, span } => {
                Expression::Await {
                    expression: Box::new(self.sub_expr(expression)),
                    span: span.clone(),
                }
            }
            Expression::OptionalMember { target, name, span } => {
                Expression::OptionalMember {
                    target: Box::new(self.sub_expr(target)),
                    name: name.clone(),
                    span: span.clone(),
                }
            }
            Expression::OptionalCall { target, arguments, span } => {
                Expression::OptionalCall {
                    target: Box::new(self.sub_expr(target)),
                    arguments: arguments.iter().map(|a| self.sub_expr(a)).collect(),
                    span: span.clone(),
                }
            }
            Expression::OptionalIndex { target, index, span } => {
                Expression::OptionalIndex {
                    target: Box::new(self.sub_expr(target)),
                    index: Box::new(self.sub_expr(index)),
                    span: span.clone(),
                }
            }
            Expression::MethodCall { object, method, type_arguments, arguments, span } => {
                Expression::MethodCall {
                    object: Box::new(self.sub_expr(object)),
                    method: method.clone(), // method name on a type, not a variable
                    type_arguments: type_arguments.clone(),
                    arguments: arguments.iter().map(|a| self.sub_expr(a)).collect(),
                    span: span.clone(),
                }
            }
            Expression::StructLiteral { name, fields, span } => {
                Expression::StructLiteral {
                    name: name.clone(), // struct type name, not a variable
                    // Field names are structural — NOT mangled; values may reference variables
                    fields: fields.iter().map(|(k, v)| (k.clone(), self.sub_expr(v))).collect(),
                    span: span.clone(),
                }
            }
            Expression::Spread { value, span } => {
                Expression::Spread {
                    value: Box::new(self.sub_expr(value)),
                    span: span.clone(),
                }
            }
            Expression::InterpolatedString { segments, span } => {
                Expression::InterpolatedString {
                    segments: segments.iter().map(|seg| match seg {
                        InterpolatedSegment::Text(t) => InterpolatedSegment::Text(t.clone()),
                        InterpolatedSegment::Expression(e) => {
                            InterpolatedSegment::Expression(self.sub_expr(e))
                        }
                    }).collect(),
                    span: span.clone(),
                }
            }
            // Literals and other leaf nodes are passed through unchanged
            other => other.clone(),
        }
    }
}

impl Default for Obfuscator {
    fn default() -> Self {
        Self::new()
    }
}
