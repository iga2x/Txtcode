/// Check if a string is a Txt-code keyword (including aliases)
pub fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        // Variable & assignment
        "store" | "let" | "const" |
        // I/O
        "print" | "out" |
        // Functions
        "define" | "def" | "return" | "ret" |
        // Control flow
        "if" | "else" | "elseif" | "elif" | "end" |
        "while" | "do" | "for" | "foreach" | "repeat" | "times" |
        "break" | "continue" |
        "match" | "switch" | "case" |
        "try" | "catch" | "finally" |
        // Logical operators
        "and" | "or" | "not" |
        // Literals
        "true" | "false" | "null" |
        // Type system
        "enum" | "struct" |
        // Module system
        "import" | "use" | "from" | "as" | "export" |
        // Permissions
        "permission" |
        // Intent declarations
        "intent" | "ai_hint" | "allowed" | "forbidden" |
        // Loop keywords
        "in" | "to" | "step" | "then" |
        // Async/await
        "async" | "await"
    )
}

/// Convert keyword alias to canonical keyword
/// Returns the canonical keyword string
pub fn canonicalize_keyword(word: &str) -> String {
    match word {
        "def" => "define".to_string(),
        "let" => "store".to_string(),
        "ret" => "return".to_string(),
        "out" => "print".to_string(),
        "use" => "import".to_string(),
        "elif" => "elseif".to_string(),
        "switch" => "match".to_string(),
        "foreach" => "for".to_string(),
        _ => word.to_string(),
    }
}

/// Get all Txt-code keywords (canonical forms only, excluding aliases)
pub fn get_keywords() -> Vec<&'static str> {
    vec![
        // Variable & assignment
        "store", "const",
        // I/O
        "print",
        // Functions
        "define", "return",
        // Control flow
        "if", "else", "elseif", "end",
        "while", "do", "for", "repeat", "times",
        "break", "continue",
        "match", "case",
        "try", "catch", "finally",
        // Logical operators
        "and", "or", "not",
        // Literals
        "true", "false", "null",
        // Type system
        "enum", "struct",
        // Module system
        "import", "export", "from", "as",
        // Loop keywords
        "in", "to", "step",
        // Async/await
        "async", "await",
    ]
}

/// Get all keyword aliases
pub fn get_keyword_aliases() -> Vec<&'static str> {
    vec![
        "let",    // alias for "store"
        "out",    // alias for "print"
        "def",    // alias for "define"
        "ret",    // alias for "return"
        "use",    // alias for "import"
        "elif",   // alias for "elseif"
        "switch", // alias for "match"
        "foreach", // alias for "for"
    ]
}

/// Check if a string is a type keyword
pub fn is_type_keyword(word: &str) -> bool {
    matches!(
        word,
        "int" | "float" | "string" | "bool" | "char" | "array" | "map"
    )
}

/// Check if a string is a reserved word (keyword or type keyword)
pub fn is_reserved(word: &str) -> bool {
    is_keyword(word) || is_type_keyword(word)
}

