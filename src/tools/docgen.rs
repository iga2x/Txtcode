use crate::lexer::Lexer;
use crate::parser::ast::*;
use crate::parser::Parser;
use std::fs;
use std::path::PathBuf;

/// A single documented item (function or struct).
#[derive(Debug, Clone)]
pub struct DocItem {
    pub kind: DocKind,
    pub name: String,
    pub params: Vec<(String, Option<String>)>, // (param_name, type_str)
    pub return_type: Option<String>,
    pub doc_comment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocKind {
    Function,
    Struct,
}

impl DocKind {
    fn as_str(&self) -> &'static str {
        match self {
            DocKind::Function => "function",
            DocKind::Struct => "struct",
        }
    }
}

/// Documentation generator
pub struct DocGenerator {
    output_format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Markdown,
    Html,
    Json,
}

impl DocGenerator {
    pub fn new() -> Self {
        Self {
            output_format: OutputFormat::Markdown,
        }
    }

    pub fn with_format(format: OutputFormat) -> Self {
        Self { output_format: format }
    }

    pub fn generate_docs(&self, program: &Program) -> String {
        let items = extract_doc_items_from_ast(program, "");
        self.render_items(&items)
    }

    /// Generate docs from raw source text. Returns the selected format.
    pub fn generate_docs_from_source(&self, source: &str) -> String {
        let items = parse_and_extract_items(source);
        self.render_items(&items)
    }

    fn render_items(&self, items: &[DocItem]) -> String {
        match self.output_format {
            OutputFormat::Markdown => render_markdown(items),
            OutputFormat::Html => markdown_to_html(&render_markdown(items)),
            OutputFormat::Json => render_json(items),
        }
    }

    /// Generate API reference docs for all `.tc` files under a stdlib directory.
    pub fn generate_stdlib_docs(
        &self,
        stdlib_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let api_dir = PathBuf::from("docs/api");
        fs::create_dir_all(&api_dir)?;

        if let Ok(entries) = fs::read_dir(stdlib_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("tc") {
                    let source = fs::read_to_string(&path)?;
                    let doc_content = self.generate_docs_from_source(&source);
                    let ext = match self.output_format {
                        OutputFormat::Html => "html",
                        OutputFormat::Json => "json",
                        _ => "md",
                    };
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("module");
                    let out_path = api_dir.join(format!("{}.{}", stem, ext));
                    fs::write(&out_path, doc_content)?;
                    println!("Generated: {}", out_path.display());
                }
            }
        }
        Ok(())
    }

    pub fn generate_html_from_source(&self, source: &str) -> String {
        markdown_to_html(&self.generate_docs_from_source(source))
    }
}

// ── Item extraction ───────────────────────────────────────────────────────────

fn parse_and_extract_items(source: &str) -> Vec<DocItem> {
    let doc_map = extract_doc_comments(source);
    let mut lexer = Lexer::new(source.to_string());
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return extract_items_from_text(source, &doc_map),
    };
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(_) => return extract_items_from_text(source, &doc_map),
    };
    extract_doc_items_from_ast(&program, source)
}

fn extract_doc_items_from_ast(program: &Program, source: &str) -> Vec<DocItem> {
    let doc_map = extract_doc_comments(source);
    let mut items = Vec::new();

    for stmt in &program.statements {
        match stmt {
            Statement::FunctionDef { name, params, return_type, intent, .. } => {
                let doc_from_intent = intent.clone().unwrap_or_default();
                let doc_from_comments = doc_map.get(name).cloned().unwrap_or_default();
                let doc_comment = if !doc_from_comments.is_empty() {
                    doc_from_comments
                } else {
                    doc_from_intent
                };

                let param_list: Vec<(String, Option<String>)> = params
                    .iter()
                    .map(|p| {
                        let ty_str = p.type_annotation.as_ref().map(|t| format!("{:?}", t));
                        (p.name.clone(), ty_str)
                    })
                    .collect();

                let ret_str = return_type.as_ref().map(|t| format!("{:?}", t));

                items.push(DocItem {
                    kind: DocKind::Function,
                    name: name.clone(),
                    params: param_list,
                    return_type: ret_str,
                    doc_comment,
                });
            }
            Statement::Struct { name, fields, .. } => {
                let doc_comment = doc_map.get(name).cloned().unwrap_or_default();
                let param_list: Vec<(String, Option<String>)> = fields
                    .iter()
                    .map(|(fname, ty)| (fname.clone(), Some(format!("{:?}", ty))))
                    .collect();

                items.push(DocItem {
                    kind: DocKind::Struct,
                    name: name.clone(),
                    params: param_list,
                    return_type: None,
                    doc_comment,
                });
            }
            _ => {}
        }
    }
    items
}

fn extract_items_from_text(
    source: &str,
    doc_map: &std::collections::HashMap<String, String>,
) -> Vec<DocItem> {
    let mut items = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("define →") || trimmed.starts_with("define ") {
            let after = trimmed
                .trim_start_matches("define")
                .trim()
                .trim_start_matches('→')
                .trim();
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                items.push(DocItem {
                    kind: DocKind::Function,
                    name: name.clone(),
                    params: vec![],
                    return_type: None,
                    doc_comment: doc_map.get(&name).cloned().unwrap_or_default(),
                });
            }
        }
    }
    items
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render_markdown(items: &[DocItem]) -> String {
    let mut md = String::new();
    md.push_str("# Txt-code Documentation\n\n");

    for item in items {
        let param_sig = item
            .params
            .iter()
            .map(|(n, t)| match t {
                Some(ty) => format!("{}: {}", n, ty),
                None => n.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");

        let kind_prefix = match item.kind {
            DocKind::Function => "",
            DocKind::Struct => "struct ",
        };
        md.push_str(&format!("## {}{}({})\n\n", kind_prefix, item.name, param_sig));

        if !item.doc_comment.is_empty() {
            md.push_str(&item.doc_comment);
            md.push_str("\n\n");
        }

        if !item.params.is_empty() {
            let p: Vec<String> = item
                .params
                .iter()
                .map(|(n, t)| match t {
                    Some(ty) => format!("{}: {}", n, ty),
                    None => n.clone(),
                })
                .collect();
            md.push_str(&format!("**Parameters:** {}\n\n", p.join(", ")));
        }

        if let Some(ret) = &item.return_type {
            md.push_str(&format!("**Returns:** {}\n\n", ret));
        }

        md.push_str("---\n\n");
    }

    md
}

fn render_json(items: &[DocItem]) -> String {
    let mut json = String::from("[\n");
    for (i, item) in items.iter().enumerate() {
        json.push_str("  {\n");
        json.push_str(&format!("    \"kind\": \"{}\",\n", item.kind.as_str()));
        json.push_str(&format!("    \"name\": \"{}\",\n", json_escape(&item.name)));
        json.push_str("    \"params\": [");
        let params_json: Vec<String> = item
            .params
            .iter()
            .map(|(n, t)| match t {
                Some(ty) => format!(
                    "{{\"name\":\"{}\",\"type\":\"{}\"}}",
                    json_escape(n),
                    json_escape(ty)
                ),
                None => format!("{{\"name\":\"{}\",\"type\":null}}", json_escape(n)),
            })
            .collect();
        json.push_str(&params_json.join(","));
        json.push_str("],\n");
        match &item.return_type {
            Some(r) => json.push_str(&format!("    \"return_type\": \"{}\",\n", json_escape(r))),
            None => json.push_str("    \"return_type\": null,\n"),
        }
        json.push_str(&format!(
            "    \"doc\": \"{}\"\n",
            json_escape(&item.doc_comment)
        ));
        json.push_str("  }");
        if i < items.len() - 1 { json.push(','); }
        json.push('\n');
    }
    json.push_str("]\n");
    json
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ── Package index ─────────────────────────────────────────────────────────────

/// Generate `docs/api/index.md` listing all packages with their exported symbols.
pub fn generate_package_index(
    packages_dir: &PathBuf,
    out_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(out_dir)?;

    let mut index = String::from("# Txt-code Package Index\n\n");

    if !packages_dir.is_dir() {
        index.push_str("No packages directory found.\n");
        fs::write(out_dir.join("index.md"), &index)?;
        return Ok(());
    }

    let mut packages: Vec<_> = fs::read_dir(packages_dir)?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    packages.sort_by_key(|e| e.file_name());

    for pkg_entry in &packages {
        let pkg_path = pkg_entry.path();
        let pkg_name = pkg_entry.file_name().to_string_lossy().into_owned();
        index.push_str(&format!("## {}\n\n", pkg_name));

        let main_tc = pkg_path.join("main.tc");
        let index_tc = pkg_path.join("index.tc");
        let src_file = if main_tc.exists() {
            Some(main_tc)
        } else if index_tc.exists() {
            Some(index_tc)
        } else {
            None
        };

        if let Some(src) = src_file {
            if let Ok(source) = fs::read_to_string(&src) {
                let items = parse_and_extract_items(&source);
                if items.is_empty() {
                    index.push_str("_No documented exports._\n\n");
                } else {
                    for item in &items {
                        let kind = match item.kind { DocKind::Function => "fn", DocKind::Struct => "struct" };
                        index.push_str(&format!("- `{}` {} ", kind, item.name));
                        if let Some(summary) = item.doc_comment.lines().next() {
                            if !summary.is_empty() {
                                index.push_str(&format!("— {}", summary));
                            }
                        }
                        index.push('\n');
                    }
                    index.push('\n');
                }
            }
        } else {
            index.push_str("_No source file found._\n\n");
        }
    }

    let out_path = out_dir.join("index.md");
    fs::write(&out_path, &index)?;
    println!("Package index written to {}", out_path.display());
    Ok(())
}

// ── Doc comment extraction ────────────────────────────────────────────────────

fn extract_doc_comments(source: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut pending_doc: Vec<&str> = Vec::new();
    let mut in_block = false;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed == "##" {
            in_block = !in_block;
            continue;
        }
        if in_block {
            pending_doc.push(trimmed);
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            pending_doc.push(stripped);
            continue;
        }

        let is_define = trimmed.starts_with("define →") || trimmed.starts_with("define ");
        let is_struct = trimmed.starts_with("struct ");

        if is_define || is_struct {
            let after = if is_define {
                trimmed.trim_start_matches("define").trim().trim_start_matches('→').trim()
            } else {
                trimmed.trim_start_matches("struct").trim()
            };
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && !pending_doc.is_empty() {
                map.insert(name, pending_doc.join("\n"));
            }
            pending_doc.clear();
            in_block = false;
            continue;
        }

        if trimmed.is_empty() {
            if !in_block { pending_doc.clear(); }
        } else {
            pending_doc.clear();
            in_block = false;
        }
    }
    map
}

// ── HTML renderer ─────────────────────────────────────────────────────────────

fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::from(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
         <style>\nbody{font-family:sans-serif;max-width:860px;margin:2rem auto;padding:0 1rem;line-height:1.6;}\n\
         h1{border-bottom:2px solid #333;padding-bottom:.3em;}\n\
         h2{border-bottom:1px solid #ccc;padding-bottom:.2em;margin-top:2em;}\n\
         code{background:#f4f4f4;padding:.1em .4em;border-radius:3px;font-size:.9em;}\n\
         pre{background:#f4f4f4;padding:1em;border-radius:4px;overflow-x:auto;}\n\
         pre code{background:none;padding:0;}\n\
         </style>\n</head>\n<body>\n",
    );

    let mut in_list = false;
    let mut in_code_block = false;
    let mut para_lines: Vec<&str> = Vec::new();

    let flush_para = |para: &mut Vec<&str>, out: &mut String| {
        if !para.is_empty() {
            let text = para.join(" ");
            if !text.trim().is_empty() {
                out.push_str(&format!("<p>{}</p>\n", inline_md(&text)));
            }
            para.clear();
        }
    };

    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            flush_para(&mut para_lines, &mut html);
            if in_code_block { html.push_str("</code></pre>\n"); in_code_block = false; }
            else { html.push_str("<pre><code>"); in_code_block = true; }
            continue;
        }
        if in_code_block { html.push_str(&html_escape_text(line)); html.push('\n'); continue; }

        if let Some(s) = line.strip_prefix("### ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            flush_para(&mut para_lines, &mut html);
            html.push_str(&format!("<h3>{}</h3>\n", inline_md(s)));
        } else if let Some(s) = line.strip_prefix("## ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            flush_para(&mut para_lines, &mut html);
            html.push_str(&format!("<h2>{}</h2>\n", inline_md(s)));
        } else if let Some(s) = line.strip_prefix("# ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            flush_para(&mut para_lines, &mut html);
            html.push_str(&format!("<h1>{}</h1>\n", inline_md(s)));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            flush_para(&mut para_lines, &mut html);
            if !in_list { html.push_str("<ul>\n"); in_list = true; }
            html.push_str(&format!("<li>{}</li>\n", inline_md(&line[2..])));
        } else if line.trim().is_empty() {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            flush_para(&mut para_lines, &mut html);
        } else {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            para_lines.push(line);
        }
    }

    if in_list { html.push_str("</ul>\n"); }
    if in_code_block { html.push_str("</code></pre>\n"); }
    flush_para(&mut para_lines, &mut html);
    html.push_str("</body>\n</html>\n");
    html
}

fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn inline_md(s: &str) -> String {
    let escaped = html_escape_text(s);
    let mut out = String::new();
    let mut in_code = false;
    for c in escaped.chars() {
        if c == '`' {
            if in_code { out.push_str("</code>"); } else { out.push_str("<code>"); }
            in_code = !in_code;
        } else {
            out.push(c);
        }
    }
    if in_code { out.push_str("</code>"); }
    out
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}
