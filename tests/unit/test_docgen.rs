use txtcode::tools::docgen::{DocGenerator, OutputFormat};

// Task 11.3 — Doc generation extracts function name from ## comments
#[test]
fn test_docgen_extracts_function_with_doc_comment() {
    let src = "## Greatest common divisor\ndefine → gcd → (a: int, b: int) → int\n  return → 0\nend";
    let gen = DocGenerator::with_format(OutputFormat::Markdown);
    let out = gen.generate_docs_from_source(src);
    assert!(out.contains("gcd"), "output should contain function name 'gcd'");
    assert!(
        out.contains("Greatest common divisor"),
        "output should contain doc comment"
    );
}

// Task 11.3 — JSON output is valid and contains expected fields
#[test]
fn test_docgen_json_output() {
    let src = "## Add two numbers\ndefine → add → (a: int, b: int) → int\n  return → 0\nend";
    let gen = DocGenerator::with_format(OutputFormat::Json);
    let out = gen.generate_docs_from_source(src);
    assert!(out.starts_with('['), "JSON output should start with '['");
    assert!(out.contains("\"name\""), "JSON should have name field");
    assert!(out.contains("\"add\""), "JSON should include function name");
    assert!(out.contains("\"doc\""), "JSON should have doc field");
}

// Task 11.3 — Markdown format includes Parameters and Returns sections
#[test]
fn test_docgen_markdown_params_and_returns() {
    let src = "define → square → (x: int) → int\n  return → 0\nend";
    let gen = DocGenerator::with_format(OutputFormat::Markdown);
    let out = gen.generate_docs_from_source(src);
    assert!(out.contains("square"), "output should include function name");
    assert!(out.contains("Parameters"), "output should include Parameters");
    assert!(out.contains("Returns"), "output should include Returns");
}

// Task 11.3 — Function with no doc comment still appears in output
#[test]
fn test_docgen_function_without_comment() {
    let src = "define → helper → () → int\n  return → 0\nend";
    let gen = DocGenerator::new();
    let out = gen.generate_docs_from_source(src);
    assert!(out.contains("helper"), "undocumented function should still appear");
}

// Task 11.3 — HTML output wraps content in html/body tags
#[test]
fn test_docgen_html_output() {
    let src = "## Hello\ndefine → greet → () → string\n  return → \"hi\"\nend";
    let gen = DocGenerator::with_format(OutputFormat::Html);
    let out = gen.generate_docs_from_source(src);
    assert!(out.contains("<html>"), "HTML output should contain <html>");
    assert!(out.contains("greet"), "HTML should include function name");
}
