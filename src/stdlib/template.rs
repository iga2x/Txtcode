use crate::runtime::{RuntimeError, Value};
use std::sync::Arc;
use indexmap::IndexMap;

/// Minimal Mustache-compatible string template engine.
///
/// Supported syntax:
/// - `{{variable}}` — variable substitution
/// - `{{#if condition}}...{{else}}...{{/if}}` — conditional (truthy check)
/// - `{{#each list as item}}...{{/each}}` — loop over array
pub struct TemplateLib;

impl TemplateLib {
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "template_render" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "template_render requires 2 arguments (template, context)".to_string(),
                    ));
                }
                let template = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new("template_render: template must be a string".to_string())),
                };
                let context = match &args[1] {
                    Value::Map(m) => m.clone(),
                    _ => return Err(RuntimeError::new("template_render: context must be a map".to_string())),
                };
                let result = render(&template, &context)
                    .map_err(|e| RuntimeError::new(format!("template_render: {}", e)))?;
                Ok(Value::String(Arc::from(result)))
            }
            _ => Err(RuntimeError::new(format!("Unknown template function: {}", name))),
        }
    }
}

// ── Renderer ─────────────────────────────────────────────────────────────────

fn render(template: &str, context: &IndexMap<String, Value>) -> Result<String, String> {
    let tokens = tokenize(template);
    render_tokens(&tokens, context)
}

#[derive(Debug, Clone)]
enum Token {
    Text(String),
    Var(String),
    IfStart(String),
    Else,
    IfEnd,
    EachStart { list: String, item: String },
    EachEnd,
}

fn tokenize(template: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = template;

    while !rest.is_empty() {
        if let Some(start) = rest.find("{{") {
            if start > 0 {
                tokens.push(Token::Text(rest[..start].to_string()));
            }
            let after_open = &rest[start + 2..];
            if let Some(end) = after_open.find("}}") {
                let tag = after_open[..end].trim();
                let tag_lower = tag.to_lowercase();
                if tag_lower == "else" {
                    tokens.push(Token::Else);
                } else if tag_lower == "/if" {
                    tokens.push(Token::IfEnd);
                } else if tag_lower == "/each" {
                    tokens.push(Token::EachEnd);
                } else if let Some(cond) = tag.strip_prefix("#if ") {
                    tokens.push(Token::IfStart(cond.trim().to_string()));
                } else if let Some(spec) = tag.strip_prefix("#each ") {
                    // Syntax: `#each list_var as item_var` or `#each list_var`
                    let (list, item) = if let Some(pos) = spec.find(" as ") {
                        (spec[..pos].trim().to_string(), spec[pos + 4..].trim().to_string())
                    } else {
                        (spec.trim().to_string(), "item".to_string())
                    };
                    tokens.push(Token::EachStart { list, item });
                } else {
                    tokens.push(Token::Var(tag.to_string()));
                }
                rest = &after_open[end + 2..];
            } else {
                // Unclosed tag — treat as text
                tokens.push(Token::Text("{{".to_string()));
                rest = after_open;
            }
        } else {
            tokens.push(Token::Text(rest.to_string()));
            break;
        }
    }
    tokens
}

fn render_tokens(tokens: &[Token], context: &IndexMap<String, Value>) -> Result<String, String> {
    let mut out = String::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Text(t) => out.push_str(t),
            Token::Var(name) => {
                out.push_str(&lookup_str(name, context));
            }
            Token::IfStart(cond) => {
                let (if_body, else_body, consumed) = collect_if_body(&tokens[i + 1..]);
                let truthy = is_truthy(context.get(cond.as_str()).unwrap_or(&Value::Null));
                let branch = if truthy { &if_body } else { &else_body };
                out.push_str(&render_tokens(branch, context)?);
                i += consumed;
            }
            Token::EachStart { list, item } => {
                let (body, consumed) = collect_each_body(&tokens[i + 1..]);
                if let Some(Value::Array(values)) = context.get(list.as_str()) {
                    for val in values {
                        let mut child_ctx = context.clone();
                        child_ctx.insert(item.clone(), val.clone());
                        // Also expose index via "loop.index" convention
                        out.push_str(&render_tokens(&body, &child_ctx)?);
                    }
                }
                i += consumed;
            }
            // These are consumed by the helpers above; if we hit them standalone it's a noop.
            Token::Else | Token::IfEnd | Token::EachEnd => {}
        }
        i += 1;
    }
    Ok(out)
}

/// Collect tokens up to the matching `{{/if}}`, splitting at `{{else}}`.
/// Returns `(if_tokens, else_tokens, total_consumed_count)`.
fn collect_if_body(tokens: &[Token]) -> (Vec<Token>, Vec<Token>, usize) {
    let mut if_body = Vec::new();
    let mut else_body = Vec::new();
    let mut depth = 0;
    let mut in_else = false;
    let mut consumed = 0;
    for (j, tok) in tokens.iter().enumerate() {
        consumed = j + 1;
        match tok {
            Token::IfStart(_) | Token::EachStart { .. } => {
                depth += 1;
                if in_else { else_body.push(tok.clone()); } else { if_body.push(tok.clone()); }
            }
            Token::IfEnd if depth == 0 => break,
            Token::IfEnd => {
                depth -= 1;
                if in_else { else_body.push(tok.clone()); } else { if_body.push(tok.clone()); }
            }
            Token::Else if depth == 0 => { in_else = true; }
            t => {
                if in_else { else_body.push(t.clone()); } else { if_body.push(t.clone()); }
            }
        }
    }
    (if_body, else_body, consumed)
}

/// Collect tokens up to matching `{{/each}}`.
/// Returns `(body_tokens, total_consumed_count)`.
fn collect_each_body(tokens: &[Token]) -> (Vec<Token>, usize) {
    let mut body = Vec::new();
    let mut depth = 0;
    let mut consumed = 0;
    for (j, tok) in tokens.iter().enumerate() {
        consumed = j + 1;
        match tok {
            Token::IfStart(_) | Token::EachStart { .. } => {
                depth += 1;
                body.push(tok.clone());
            }
            Token::EachEnd if depth == 0 => break,
            Token::EachEnd => {
                depth -= 1;
                body.push(tok.clone());
            }
            t => body.push(t.clone()),
        }
    }
    (body, consumed)
}

fn lookup_str(name: &str, context: &IndexMap<String, Value>) -> String {
    match context.get(name) {
        Some(v) => value_to_string(v),
        None => String::new(),
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.to_string(),
        other => other.to_string(),
    }
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Boolean(b) => *b,
        Value::Integer(0) => false,
        Value::Integer(_) => true,
        Value::Float(f) => *f != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Map(m) => !m.is_empty(),
        _ => true,
    }
}
