use crate::parser::ast::*;

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

    fn generate_markdown(&self, program: &Program) -> String {
        let mut docs = String::new();
        docs.push_str("# Txt-code Documentation\n\n");

        for statement in &program.statements {
            if let Statement::FunctionDef { name, params, return_type, .. } = statement {
                docs.push_str(&format!("## Function: `{}`\n\n", name));
                
                if !params.is_empty() {
                    docs.push_str("### Parameters\n\n");
                    for param in params {
                        docs.push_str(&format!("- `{}`", param.name));
                        if let Some(ty) = &param.type_annotation {
                            docs.push_str(&format!(": {:?}", ty));
                        }
                        docs.push_str("\n");
                    }
                    docs.push_str("\n");
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
        // In a full implementation, convert markdown to HTML
        format!("<html><body><pre>{}</pre></body></html>", markdown)
    }
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

