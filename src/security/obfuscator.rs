// AST-level identifier obfuscation — intentional no-op stub.
//
// STATUS: STUB — `obfuscate()` returns the program unchanged.
//
// Do not rely on this for IP protection in v0.4. The Obfuscator type is exported
// so the API is stable when the implementation is added.
//
// Planned techniques (not yet implemented):
//   - Variable/function identifier mangling
//   - String literal splitting and re-joining
//   - Dead-code insertion
//   - Control-flow flattening

use crate::parser::ast::Program;

/// AST-level code obfuscator. No-op stub in v0.4.
pub struct Obfuscator;

impl Obfuscator {
    pub fn new() -> Self {
        Self
    }

    /// Returns `program` unchanged. Obfuscation is not yet implemented.
    pub fn obfuscate(&mut self, program: &Program) -> Program {
        program.clone()
    }
}

impl Default for Obfuscator {
    fn default() -> Self {
        Self::new()
    }
}
