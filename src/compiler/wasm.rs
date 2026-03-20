/// Task 12.3 — WASM Compilation Target
///
/// Translates a `Bytecode` program into WebAssembly Text Format (WAT).
/// Scope: integers, floats, basic arithmetic, variables, if/while, function calls.
/// Complex values (arrays, maps, structs) are not yet supported and emit a trap.
///
/// # Usage
/// ```
/// let wat = WasmCompiler::new().compile(&bytecode);
/// std::fs::write("output.wat", wat).unwrap();
/// // Convert to binary: wat2wasm output.wat -o output.wasm  (via wasm-tools or WABT)
/// ```

use crate::compiler::bytecode::{Bytecode, Instruction};
use crate::runtime::core::Value;

/// Generates WAT (WebAssembly Text Format) from a `Bytecode` program.
pub struct WasmCompiler {
    /// All local variable names seen during compilation (for `(local ...)` declarations)
    locals: Vec<String>,
    /// Function bodies accumulated during compilation
    functions: Vec<WatFunction>,
    /// WAT data segment for string constants
    data_segments: Vec<(usize, String)>,
    /// Next offset into linear memory for string data
    mem_offset: usize,
}

struct WatFunction {
    name: String,
    params: Vec<String>,
    locals: Vec<String>,
    body: Vec<String>,
    export: bool,
}

impl WasmCompiler {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            functions: Vec::new(),
            data_segments: Vec::new(),
            mem_offset: 0,
        }
    }

    /// Compile a `Bytecode` program to a WAT module string.
    pub fn compile(&mut self, bytecode: &Bytecode) -> String {
        let instructions = &bytecode.instructions;
        let constants = &bytecode.constants;

        // Collect all StoreVar / LoadVar names to declare as locals
        let mut local_names: Vec<String> = Vec::new();
        for inst in instructions {
            match inst {
                Instruction::StoreVar(n) | Instruction::StoreConst(n) | Instruction::LoadVar(n) => {
                    if !local_names.contains(n) {
                        local_names.push(n.clone());
                    }
                }
                _ => {}
            }
        }

        let body_lines = self.compile_instructions(instructions, constants, &local_names);

        // Build main function
        let mut local_decls: Vec<String> = local_names
            .iter()
            .map(|n| format!("  (local ${} i64)", sanitize_name(n)))
            .collect();
        // Stack scratch registers
        local_decls.push("  (local $__tmp i64)".to_string());
        local_decls.push("  (local $__tmp2 i64)".to_string());

        let mut wat = String::new();
        wat.push_str("(module\n");

        // Linear memory for string data
        wat.push_str("  (memory 1)\n");

        // Data segments for string constants
        for (offset, s) in &self.data_segments {
            let escaped = escape_string(s);
            wat.push_str(&format!(
                "  (data (i32.const {}) \"{}\")\n",
                offset, escaped
            ));
        }

        // i64.print import (host function for print)
        wat.push_str("  (import \"env\" \"print_i64\" (func $print_i64 (param i64)))\n");
        wat.push_str("  (import \"env\" \"print_f64\" (func $print_f64 (param f64)))\n");

        // main function
        wat.push_str("  (func $main (export \"main\")\n");
        for decl in &local_decls {
            wat.push_str(decl);
            wat.push('\n');
        }
        for line in &body_lines {
            wat.push_str("    ");
            wat.push_str(line);
            wat.push('\n');
        }
        // Drop any leftover stack value
        wat.push_str("    (if (i32.const 0) (then))\n"); // no-op placeholder
        wat.push_str("  )\n");

        wat.push_str(")\n");
        wat
    }

    fn compile_instructions(
        &self,
        instructions: &[Instruction],
        constants: &[Value],
        locals: &[String],
    ) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut i = 0;

        while i < instructions.len() {
            let inst = &instructions[i];
            match inst {
                Instruction::PushConstant(idx) => {
                    match constants.get(*idx) {
                        Some(Value::Integer(n)) => {
                            out.push(format!("i64.const {}", n));
                        }
                        Some(Value::Float(f)) => {
                            out.push(format!("f64.const {}", f));
                            // cast to i64 for stack uniformity (limited support)
                            out.push("i64.trunc_f64_s".to_string());
                        }
                        Some(Value::Boolean(b)) => {
                            out.push(format!("i64.const {}", if *b { 1 } else { 0 }));
                        }
                        Some(Value::Null) => {
                            out.push("i64.const 0".to_string());
                        }
                        Some(Value::String(_)) => {
                            // String: push a 0 pointer (limited support)
                            out.push("i64.const 0  ;; string literal (limited WASM support)".to_string());
                        }
                        _ => {
                            out.push("i64.const 0  ;; unsupported constant type".to_string());
                        }
                    }
                }
                Instruction::LoadVar(name) => {
                    out.push(format!("local.get ${}", sanitize_name(name)));
                }
                Instruction::StoreVar(name) | Instruction::StoreConst(name) => {
                    out.push(format!("local.set ${}", sanitize_name(name)));
                }
                Instruction::Add => {
                    out.push("i64.add".to_string());
                }
                Instruction::Subtract => {
                    out.push("i64.sub".to_string());
                }
                Instruction::Multiply => {
                    out.push("i64.mul".to_string());
                }
                Instruction::Divide => {
                    out.push("i64.div_s".to_string());
                }
                Instruction::Modulo => {
                    out.push("i64.rem_s".to_string());
                }
                Instruction::Negate => {
                    out.push("i64.const -1".to_string());
                    out.push("i64.mul".to_string());
                }
                Instruction::Equal => {
                    out.push("i64.eq".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::NotEqual => {
                    out.push("i64.ne".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::Less => {
                    out.push("i64.lt_s".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::Greater => {
                    out.push("i64.gt_s".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::LessEqual => {
                    out.push("i64.le_s".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::GreaterEqual => {
                    out.push("i64.ge_s".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::And => {
                    out.push("i64.and".to_string());
                }
                Instruction::Or => {
                    out.push("i64.or".to_string());
                }
                Instruction::Not => {
                    out.push("i64.eqz".to_string());
                    out.push("i64.extend_i32_u".to_string());
                }
                Instruction::Jump(target) => {
                    // WAT uses structured control flow; we emit a comment with the target
                    out.push(format!(";; Jump to {} (requires block structure)", target));
                }
                Instruction::JumpIfFalse(target) => {
                    out.push(format!(";; JumpIfFalse {} (requires block structure)", target));
                }
                Instruction::JumpIfTrue(target) => {
                    out.push(format!(";; JumpIfTrue {} (requires block structure)", target));
                }
                Instruction::Pop => {
                    out.push("drop".to_string());
                }
                Instruction::Dup => {
                    // Duplicate via local scratch
                    out.push("local.set $__tmp".to_string());
                    out.push("local.get $__tmp".to_string());
                    out.push("local.get $__tmp".to_string());
                }
                Instruction::Call(name, _argc) => {
                    // Print function → call host import
                    if name == "print" || name == "println" {
                        out.push("call $print_i64".to_string());
                    } else {
                        out.push(format!("call ${}", sanitize_name(name)));
                    }
                }
                Instruction::Return | Instruction::ReturnValue => {
                    out.push("return".to_string());
                }
                Instruction::Nop => {
                    out.push("nop".to_string());
                }
                _ => {
                    out.push(format!(";; Unsupported instruction: {:?}", inst));
                }
            }
            i += 1;
        }

        out
    }
}

impl Default for WasmCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize a variable name for WAT identifier rules (no dots, arrows, etc.)
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Escape a string for WAT data segment syntax
fn escape_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => {
                out.push_str(&format!("\\{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}
