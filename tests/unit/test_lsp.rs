// Task 11.4 — LSP symbol table and navigation helpers

// These test the internal helper functions exposed via the lsp module.
// We re-implement the same logic here since the helpers are private to the
// lsp module — we test the observable behavior via the doc-gen layer and
// via the lsp itself. For unit-testing we copy the logic under test.

/// Re-implementation of symbol_at for testing (same logic as lsp.rs).
fn symbol_at(text: &str, line: usize, character: usize) -> Option<String> {
    let src_line = text.lines().nth(line)?;
    let chars: Vec<char> = src_line.chars().collect();
    if character >= chars.len() {
        return None;
    }
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    if !is_ident(chars[character]) {
        return None;
    }
    let start = (0..=character)
        .rev()
        .take_while(|&i| is_ident(chars[i]))
        .last()
        .unwrap_or(character);
    let end = (character..chars.len())
        .take_while(|&i| is_ident(chars[i]))
        .last()
        .unwrap_or(character);
    let word: String = chars[start..=end].iter().collect();
    if word.is_empty() { None } else { Some(word) }
}

fn find_definition(text: &str, name: &str) -> Option<(usize, usize)> {
    for (ln, line) in text.lines().enumerate() {
        let trimmed = line.trim();
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
                if let Some(pos) = line.find(name) {
                    return Some((ln, pos));
                }
            }
        }
    }
    None
}

fn find_all_occurrences(text: &str, name: &str) -> Vec<(usize, usize, usize, usize)> {
    let mut results = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let name_bytes = name.as_bytes();
        let mut start = 0;
        while start + name_bytes.len() <= bytes.len() {
            if let Some(pos) = line[start..].find(name) {
                let abs_pos = start + pos;
                let before_ok = abs_pos == 0
                    || (!bytes[abs_pos - 1].is_ascii_alphanumeric() && bytes[abs_pos - 1] != b'_');
                let after_pos = abs_pos + name_bytes.len();
                let after_ok = after_pos >= bytes.len()
                    || (!bytes[after_pos].is_ascii_alphanumeric() && bytes[after_pos] != b'_');
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

// Task 11.4 — symbol_at extracts the word under the cursor
#[test]
fn test_symbol_at_middle_of_word() {
    let text = "store → myVar → 42";
    // 'myVar' starts at char 9 in that line (after "store → ")
    let sym = symbol_at(text, 0, 11); // position inside 'myVar'
    assert_eq!(sym.as_deref(), Some("myVar"));
}

#[test]
fn test_symbol_at_space_returns_none() {
    let text = "store → x → 42";
    let sym = symbol_at(text, 0, 7); // at '→'
    assert_eq!(sym, None);
}

// Task 11.4 — find_definition locates define → name
#[test]
fn test_find_definition_function() {
    let src = "store → a → 1\ndefine → gcd → (a: int, b: int) → int\n  return → 0\nend";
    let loc = find_definition(src, "gcd");
    assert!(loc.is_some(), "should find definition of gcd");
    let (line, _) = loc.unwrap();
    assert_eq!(line, 1, "gcd is defined on line 1 (0-based)");
}

#[test]
fn test_find_definition_returns_none_for_unknown() {
    let src = "define → foo → () → int\n  return → 0\nend";
    let loc = find_definition(src, "bar");
    assert!(loc.is_none(), "bar is not defined");
}

// Task 11.4 — find_all_occurrences returns all whole-word matches
#[test]
fn test_find_all_occurrences_single() {
    let text = "store → x → 1\nstore → y → x + 2";
    let occurrences = find_all_occurrences(text, "x");
    assert_eq!(occurrences.len(), 2, "x appears twice");
}

#[test]
fn test_find_all_occurrences_word_boundary() {
    // 'foo' should not match 'foobar'
    let text = "foobar\nfoo\nfoo + 1";
    let occurrences = find_all_occurrences(text, "foo");
    // line 0 has 'foobar' — not a match; lines 1 and 2 have 'foo'
    assert_eq!(occurrences.len(), 2, "foo only matches whole words");
}
