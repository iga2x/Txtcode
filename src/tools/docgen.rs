use crate::parser::ast::*;
use std::fs;
use std::path::PathBuf;

/// Documentation generator
pub struct DocGenerator {
    output_format: OutputFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Markdown,
    Html,
}

impl DocGenerator {
    pub fn new() -> Self {
        Self {
            output_format: OutputFormat::Markdown,
        }
    }

    pub fn with_format(format: OutputFormat) -> Self {
        Self {
            output_format: format,
        }
    }

    pub fn generate_docs(&self, program: &Program) -> String {
        match self.output_format {
            OutputFormat::Markdown => self.generate_markdown(program),
            OutputFormat::Html => self.generate_html(program),
        }
    }

    /// Generate docs from raw source text, extracting `##` doc comments that
    /// precede `define` statements. Returns Markdown or HTML depending on format.
    pub fn generate_docs_from_source(&self, source: &str) -> String {
        let doc_map = extract_doc_comments(source);
        let mut md = String::new();
        md.push_str("# Txt-code Documentation\n\n");

        let mut names: Vec<&String> = doc_map.keys().collect();
        names.sort();
        for name in names {
            let comment = &doc_map[name];
            md.push_str(&format!("## Function: `{}`\n\n", name));
            if !comment.is_empty() {
                md.push_str(comment);
                md.push_str("\n\n");
            }
        }

        match self.output_format {
            OutputFormat::Html => markdown_to_html(&md),
            OutputFormat::Markdown => md,
        }
    }

    /// Generate API reference docs for all `.tc` files under a stdlib directory.
    /// Outputs to `docs/api/` relative to the current directory.
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

                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("module");
                    let out_path = api_dir.join(format!("{}.md", stem));
                    fs::write(&out_path, doc_content)?;
                    println!("Generated: {}", out_path.display());
                }
            }
        }

        Ok(())
    }

    fn generate_markdown(&self, program: &Program) -> String {
        let mut docs = String::new();
        docs.push_str("# Txt-code Documentation\n\n");

        let iter = program.statements.iter().peekable();
        for statement in iter {
            if let Statement::FunctionDef {
                name,
                params,
                return_type,
                intent,
                ai_hint,
                ..
            } = statement
            {
                docs.push_str(&format!("## Function: `{}`\n\n", name));

                // doc → "description" (shown as the function description)
                if let Some(doc_text) = intent {
                    docs.push_str(&format!("{}\n\n", doc_text));
                }
                // hint → "usage hint" (shown as a hint box for humans and tools)
                if let Some(hint_text) = ai_hint {
                    docs.push_str(&format!("**Hint:** {}\n\n", hint_text));
                }

                if !params.is_empty() {
                    docs.push_str("### Parameters\n\n");
                    for param in params {
                        docs.push_str(&format!("- `{}`", param.name));
                        if let Some(ty) = &param.type_annotation {
                            docs.push_str(&format!(": {:?}", ty));
                        }
                        docs.push('\n');
                    }
                    docs.push('\n');
                }

                if let Some(ty) = return_type {
                    docs.push_str(&format!("### Returns\n\n`{:?}`\n\n", ty));
                }
            }
        }

        docs
    }

    fn generate_html(&self, program: &Program) -> String {
        let markdown = self.generate_markdown(program);
        markdown_to_html(&markdown)
    }

    /// Generate HTML directly from source (used by generate_docs_from_source for html format).
    pub fn generate_html_from_source(&self, source: &str) -> String {
        let markdown = self.generate_docs_from_source(source);
        markdown_to_html(&markdown)
    }
}

/// Extract `##`-style doc comments that appear immediately before `define` lines.
/// Returns a map of function_name → doc_comment_text.
fn extract_doc_comments(source: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut pending_doc: Vec<&str> = Vec::new();
    let mut in_block = false;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed == "##" {
            // Toggle multi-line doc comment block
            in_block = !in_block;
            continue;
        }

        if in_block {
            pending_doc.push(trimmed);
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix("## ") {
            // Single-line doc comment (not a block toggle)
            pending_doc.push(stripped);
            continue;
        }

        // Check for `define →` or `define ` lines to attach doc comment
        if trimmed.starts_with("define →") || trimmed.starts_with("define ") {
            let after = trimmed
                .trim_start_matches("define")
                .trim_start_matches(" →")
                .trim_start_matches(" ")
                .trim_start_matches("→")
                .trim();
            // Extract function name (first token before space or `(`)
            let func_name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();

            if !func_name.is_empty() && !pending_doc.is_empty() {
                map.insert(func_name, pending_doc.join("\n"));
            }
            pending_doc.clear();
            in_block = false;
            continue;
        }

        // Blank lines clear the pending doc comment (doc comments must immediately
        // precede the `define` they document, with no intervening blank line).
        if trimmed.is_empty() {
            if !in_block {
                pending_doc.clear();
            }
        } else {
            // Any other non-doc, non-define line resets state
            pending_doc.clear();
            in_block = false;
        }
    }

    map
}

/// Convert a simple subset of Markdown to HTML.
/// Handles: # headings, ## headings, ### headings, - bullet lists,
/// ``` code blocks, blank paragraphs, and inline `code`.
fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<style>\nbody{font-family:sans-serif;max-width:860px;margin:2rem auto;padding:0 1rem;line-height:1.6;}\n");
    html.push_str("h1{border-bottom:2px solid #333;padding-bottom:.3em;}\n");
    html.push_str("h2{border-bottom:1px solid #ccc;padding-bottom:.2em;margin-top:2em;}\n");
    html.push_str("code{background:#f4f4f4;padding:.1em .4em;border-radius:3px;font-size:.9em;}\n");
    html.push_str("pre{background:#f4f4f4;padding:1em;border-radius:4px;overflow-x:auto;}\n");
    html.push_str("pre code{background:none;padding:0;}\n");
    html.push_str("</style>\n</head>\n<body>\n");

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
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            flush_para(&mut para_lines, &mut html);
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                html.push_str("<pre><code>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape_text(line));
            html.push('\n');
            continue;
        }

        if let Some(stripped) = line.strip_prefix("### ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            flush_para(&mut para_lines, &mut html);
            html.push_str(&format!("<h3>{}</h3>\n", inline_md(stripped)));
        } else if let Some(stripped) = line.strip_prefix("## ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            flush_para(&mut para_lines, &mut html);
            html.push_str(&format!("<h2>{}</h2>\n", inline_md(stripped)));
        } else if let Some(stripped) = line.strip_prefix("# ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            flush_para(&mut para_lines, &mut html);
            html.push_str(&format!("<h1>{}</h1>\n", inline_md(stripped)));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            flush_para(&mut para_lines, &mut html);
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>\n", inline_md(&line[2..])));
        } else if line.trim().is_empty() {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            flush_para(&mut para_lines, &mut html);
        } else {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            para_lines.push(line);
        }
    }

    if in_list {
        html.push_str("</ul>\n");
    }
    if in_code_block {
        html.push_str("</code></pre>\n");
    }
    flush_para(&mut para_lines, &mut html);

    html.push_str("</body>\n</html>\n");
    html
}

fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Process inline markdown: `code`, **bold** → proper HTML.
fn inline_md(s: &str) -> String {
    let escaped = html_escape_text(s);
    // Backtick code spans
    let mut out = String::new();
    let mut in_code = false;
    let chars = escaped.chars().peekable();
    for c in chars {
        if c == '`' {
            if in_code {
                out.push_str("</code>");
            } else {
                out.push_str("<code>");
            }
            in_code = !in_code;
        } else {
            out.push(c);
        }
    }
    if in_code {
        out.push_str("</code>");
    }
    out
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}
