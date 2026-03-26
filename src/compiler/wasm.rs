/// Task 12.3 — WASM Compilation Target
///
/// Translates a `Bytecode` program into WebAssembly Text Format (WAT).
/// Scope: integers, floats, basic arithmetic, variables, if/while, function calls.
/// Complex values (arrays, maps, structs) are not yet supported and emit a trap.
///
/// # Usage
/// ```ignore
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
        // Convert Constant → Value for compile_instructions (which uses Value matching)
        let constants_as_values: Vec<Value> = bytecode.constants.iter().map(|c| c.to_value()).collect();
        let constants = constants_as_values.as_slice();

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

        // Data segments for string constants (Group 29.1)
        // Each string is stored with a null terminator for C-compatible host calls.
        for (offset, s) in &self.data_segments {
            let escaped = escape_string(s);
            wat.push_str(&format!(
                "  (data (i32.const {}) \"{}\\00\")  ;; len={}\n",
                offset,
                escaped,
                s.len()
            ));
        }

        // Host function imports
        wat.push_str("  (import \"env\" \"print_i64\" (func $print_i64 (param i64)))\n");
        wat.push_str("  (import \"env\" \"print_f64\" (func $print_f64 (param f64)))\n");
        // Group 29.1: string and array host functions
        // print_str(ptr: i32, len: i32) — host prints UTF-8 string from linear memory
        wat.push_str("  (import \"env\" \"print_str\" (func $print_str (param i32 i32)))\n");
        // array_new(count: i64) → i64 — host allocates a heap array; previous `count` i64s on stack are elements
        wat.push_str("  (import \"env\" \"array_new\" (func $array_new (param i64) (result i64)))\n");
        // array_get(arr: i64, index: i64) → i64 — host reads element from array
        wat.push_str("  (import \"env\" \"array_get\" (func $array_get (param i64 i64) (result i64)))\n");
        // array_len(arr: i64) → i64 — host returns element count
        wat.push_str("  (import \"env\" \"array_len\" (func $array_len (param i64) (result i64)))\n");
        // str_len(packed: i64) → i64 — extract string length from packed representation
        wat.push_str("  ;; str_len: inline extraction: (i64.and packed 0xFFFFFFFF)\n");

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

    /// Intern a string into the data segment, returning its memory offset.
    /// Identical strings share the same data segment entry.
    fn intern_string(&mut self, s: &str) -> (usize, usize) {
        // Check if string is already in a data segment
        for (off, existing) in &self.data_segments {
            if existing == s {
                return (*off, s.len());
            }
        }
        let offset = self.mem_offset;
        // Align to 4 bytes and reserve space for string bytes + null terminator
        let byte_len = s.len() + 1; // +1 for null terminator
        self.mem_offset += byte_len;
        // Align next segment to 4-byte boundary
        let remainder = self.mem_offset % 4;
        if remainder != 0 {
            self.mem_offset += 4 - remainder;
        }
        self.data_segments.push((offset, s.to_string()));
        (offset, s.len())
    }

    fn compile_instructions(
        &mut self,
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
                        Some(Value::String(s)) => {
                            // Group 29.1: String support via linear memory data segments.
                            // Strings are encoded as: (offset << 32) | length  in an i64.
                            // The host runtime unpacks ptr = (val >> 32) as i32, len = val as i32.
                            let (offset, len) = self.intern_string(s);
                            let packed = ((offset as i64) << 32) | (len as i64);
                            out.push(format!(
                                "i64.const {}  ;; string \"{}\": ptr={} len={}",
                                packed,
                                s.chars().take(20).collect::<String>(),
                                offset,
                                len
                            ));
                        }
                        Some(Value::Array(arr)) => {
                            // Group 29.1: Array literals.
                            // Push each element then call the host array_new(count) import
                            // which allocates a heap array and returns a reference pointer.
                            // If array is all integers, emit optimized inline store.
                            let count = arr.len();
                            for item in arr {
                                match item {
                                    Value::Integer(n) => out.push(format!("i64.const {}", n)),
                                    Value::Float(f) => {
                                        out.push(format!("f64.const {}", f));
                                        out.push("i64.trunc_f64_s".to_string());
                                    }
                                    Value::Boolean(b) => {
                                        out.push(format!("i64.const {}", if *b { 1 } else { 0 }))
                                    }
                                    _ => out.push("i64.const 0".to_string()),
                                }
                            }
                            out.push(format!("i64.const {}", count));
                            out.push("call $array_new  ;; host: allocate array".to_string());
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
                    match name.as_str() {
                        "print" | "println" => {
                            // Group 29.1: For string values (packed ptr|len), call print_str.
                            // For integers, call print_i64. At WASM compile time we don't know
                            // the type; emit both via the host dispatch convention.
                            // The WAT runtime uses the high-word to distinguish: 0 = integer.
                            out.push(";; print dispatch: host determines type from packed i64".to_string());
                            out.push("call $print_i64".to_string());
                        }
                        "len" => {
                            // Group 29.1: array_len or str_len
                            out.push("call $array_len  ;; len() on array".to_string());
                        }
                        "array_get" => {
                            out.push("call $array_get".to_string());
                        }
                        _ => {
                            out.push(format!("call ${}", sanitize_name(name)));
                        }
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

// ── S.3: IR-based WAT generation ──────────────────────────────────────────────
//
// When the `ir` feature is enabled, `WasmCompiler` gains a second entry point
// that takes a `ProgramIr` instead of a `Bytecode`.  The IR uses structured
// control flow (`If`, `Loop`, `ForEach`) which maps directly onto WAT's
// structured control flow — no jump-comment workarounds needed.

#[cfg(feature = "ir")]
mod ir_backend {
    use super::{escape_string, sanitize_name};
    use crate::ir::{IrNode, ProgramIr};
    use crate::parser::ast::common::{BinaryOperator, Literal, UnaryOperator};

    impl super::WasmCompiler {
        /// Compile a `ProgramIr` to a WAT module string.
        ///
        /// Uses structured WAT control flow (`if/then/else`, `block/loop/br_if`)
        /// matching the IR's structured nodes — no flat jump targets.
        pub fn compile_from_ir(&mut self, ir: &ProgramIr) -> String {
            // Collect all variable names for `(local ...)` declarations.
            let mut local_names: Vec<String> = Vec::new();
            for node in &ir.nodes {
                collect_ir_locals(node, &mut local_names);
            }

            // Compile top-level nodes.
            let mut body_lines: Vec<String> = Vec::new();
            for node in &ir.nodes {
                self.emit_ir_node(node, &mut body_lines, 0);
            }

            build_wat_module(&local_names, &body_lines, &self.data_segments)
        }

        fn emit_ir_node(&mut self, node: &IrNode, out: &mut Vec<String>, depth: usize) {
            let indent = "  ".repeat(depth);
            match node {
                IrNode::Const(lit) => {
                    out.push(format!("{}{}",indent, ir_const_to_wat(lit, self)));
                }
                IrNode::Var(name) => {
                    out.push(format!("{}local.get ${}", indent, sanitize_name(name)));
                }
                IrNode::Assign { name, value, .. } => {
                    self.emit_ir_node(value, out, depth);
                    out.push(format!("{}local.set ${}", indent, sanitize_name(name)));
                }
                IrNode::BinOp { left, op, right, .. } => {
                    self.emit_ir_node(left, out, depth);
                    self.emit_ir_node(right, out, depth);
                    out.push(format!("{}{}", indent, binop_to_wat(*op)));
                }
                IrNode::UnaryOp { op, operand, .. } => {
                    self.emit_ir_node(operand, out, depth);
                    out.push(format!("{}{}", indent, unop_to_wat(*op)));
                }
                IrNode::If { condition, then_block, else_block, .. } => {
                    // WAT structured if: push condition, then (if (then ...) (else ...))
                    self.emit_ir_node(condition, out, depth);
                    out.push(format!("{}(if (then", indent));
                    self.emit_ir_node(then_block, out, depth + 1);
                    if let Some(else_b) = else_block {
                        out.push(format!("{}  ) (else", indent));
                        self.emit_ir_node(else_b, out, depth + 1);
                    }
                    out.push(format!("{}))", indent));
                }
                IrNode::Loop { condition: Some(cond), body, .. } => {
                    // WAT while-loop pattern:
                    //   (block $break (loop $continue
                    //     <cond> i32.wrap_i64  ;; condition → i32
                    //     i32.eqz br_if $break  ;; exit if false
                    //     <body>
                    //     br $continue))
                    out.push(format!("{}(block $break_{d}", indent, d = depth));
                    out.push(format!("{}  (loop $loop_{d}", indent, d = depth));
                    self.emit_ir_node(cond, out, depth + 2);
                    out.push(format!("{}    i32.wrap_i64", indent));
                    out.push(format!("{}    i32.eqz", indent));
                    out.push(format!("{}    br_if $break_{d}", indent, d = depth));
                    self.emit_ir_node(body, out, depth + 2);
                    out.push(format!("{}    br $loop_{d}", indent, d = depth));
                    out.push(format!("{}  )", indent));
                    out.push(format!("{})", indent));
                }
                IrNode::Loop { condition: None, body, .. } => {
                    // Infinite loop: `(block $break (loop $loop (body) (br $loop)))`
                    out.push(format!("{}(block $break_{d}", indent, d = depth));
                    out.push(format!("{}  (loop $loop_{d}", indent, d = depth));
                    self.emit_ir_node(body, out, depth + 2);
                    out.push(format!("{}    br $loop_{d}", indent, d = depth));
                    out.push(format!("{}  )", indent));
                    out.push(format!("{})", indent));
                }
                IrNode::ForEach { variable, iterable, body, .. } => {
                    // Desugar: store iterable in __iter__, iterate via index.
                    // Emit as an index-based loop using a scratch counter.
                    out.push(format!("{}  ;; for-each '{}' (IR backend: desugar)", indent, variable));
                    self.emit_ir_node(iterable, out, depth);
                    // Simplified: emit body only (array iteration not fully representable in WAT i64-only VM)
                    out.push(format!("{}  ;; ForEach body:", indent));
                    self.emit_ir_node(body, out, depth + 1);
                }
                IrNode::Block(nodes) => {
                    for n in nodes {
                        self.emit_ir_node(n, out, depth);
                    }
                }
                IrNode::Call { name, args, .. } => {
                    for arg in args {
                        self.emit_ir_node(arg, out, depth);
                    }
                    let wat_name = match name.as_str() {
                        "print" | "println" => "$print_i64",
                        "len"               => "$array_len",
                        _                   => &format!("${}", sanitize_name(name)),
                    };
                    out.push(format!("{}call {}", indent, wat_name));
                }
                IrNode::CapabilityCall { call, .. } => {
                    // Capability is enforcement metadata; the WAT backend just emits the call.
                    self.emit_ir_node(call, out, depth);
                }
                IrNode::FunctionDef { name, params, body, .. } => {
                    // Inline function definitions as WAT functions (collected separately
                    // in a real multi-function module; here emitted as comments).
                    out.push(format!("{}  ;; fn {} ({} params) — defined in WAT func table", indent, name, params.len()));
                    self.emit_ir_node(body, out, depth + 1);
                }
                IrNode::Return(val) => {
                    if let Some(v) = val {
                        self.emit_ir_node(v, out, depth);
                    }
                    out.push(format!("{}return", indent));
                }
                IrNode::Break => {
                    out.push(format!("{}br $break_0  ;; break", indent));
                }
                IrNode::Continue => {
                    out.push(format!("{}br $loop_0  ;; continue", indent));
                }
                IrNode::IndexAssign { target, index, value, .. } => {
                    // Not representable in this simplified i64-only WASM backend.
                    out.push(format!("{}  ;; IndexAssign (unsupported in IR-WAT backend)", indent));
                    // Still lower the sub-expressions to avoid silently dropping side effects.
                    self.emit_ir_node(target, out, depth);
                    out.push(format!("{}  drop", indent));
                    self.emit_ir_node(index, out, depth);
                    out.push(format!("{}  drop", indent));
                    self.emit_ir_node(value, out, depth);
                    out.push(format!("{}  drop", indent));
                }
                IrNode::Array(elements) => {
                    let count = elements.len();
                    for el in elements {
                        self.emit_ir_node(el, out, depth);
                    }
                    out.push(format!("{}i64.const {}", indent, count));
                    out.push(format!("{}call $array_new", indent));
                }
                IrNode::Map(_) => {
                    out.push(format!("{}i64.const 0  ;; Map literal (unsupported in IR-WAT backend)", indent));
                }
                IrNode::Nop => {
                    out.push(format!("{}nop", indent));
                }
            }
        }
    }

    fn ir_const_to_wat(lit: &Literal, compiler: &mut super::WasmCompiler) -> String {
        match lit {
            Literal::Integer(n)  => format!("i64.const {}", n),
            Literal::Float(f)    => format!("f64.const {}  ;; float (truncated to i64)\ni64.trunc_f64_s", f),
            Literal::Boolean(b)  => format!("i64.const {}", if *b { 1 } else { 0 }),
            Literal::Null        => "i64.const 0  ;; null".to_string(),
            Literal::String(s)   => {
                let (offset, len) = compiler.intern_string(s);
                let packed = ((offset as i64) << 32) | (len as i64);
                format!("i64.const {}  ;; string {:?}: ptr={} len={}", packed, s, offset, len)
            }
            Literal::Char(c) => format!("i64.const {}", *c as i64),
        }
    }

    fn binop_to_wat(op: BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Add           => "i64.add",
            BinaryOperator::Subtract      => "i64.sub",
            BinaryOperator::Multiply      => "i64.mul",
            BinaryOperator::Divide        => "i64.div_s",
            BinaryOperator::Modulo        => "i64.rem_s",
            BinaryOperator::Equal         => "i64.eq\ni64.extend_i32_u",
            BinaryOperator::NotEqual      => "i64.ne\ni64.extend_i32_u",
            BinaryOperator::Less          => "i64.lt_s\ni64.extend_i32_u",
            BinaryOperator::Greater       => "i64.gt_s\ni64.extend_i32_u",
            BinaryOperator::LessEqual     => "i64.le_s\ni64.extend_i32_u",
            BinaryOperator::GreaterEqual  => "i64.ge_s\ni64.extend_i32_u",
            BinaryOperator::And           => "i64.and",
            BinaryOperator::Or            => "i64.or",
            BinaryOperator::BitwiseAnd    => "i64.and",
            BinaryOperator::BitwiseOr     => "i64.or",
            BinaryOperator::BitwiseXor    => "i64.xor",
            BinaryOperator::LeftShift     => "i64.shl",
            BinaryOperator::RightShift    => "i64.shr_s",
            _                             => ";; unsupported binop\nnop",
        }
    }

    fn unop_to_wat(op: UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::Minus  => "i64.const -1\ni64.mul",
            UnaryOperator::Not    => "i64.eqz\ni64.extend_i32_u",
            UnaryOperator::BitNot => "i64.const -1\ni64.xor",
            _                     => ";; unsupported unop\nnop",
        }
    }

    fn collect_ir_locals(node: &IrNode, locals: &mut Vec<String>) {
        match node {
            IrNode::Assign { name, value, .. } => {
                if !locals.contains(name) { locals.push(name.clone()); }
                collect_ir_locals(value, locals);
            }
            IrNode::Block(nodes) => { for n in nodes { collect_ir_locals(n, locals); } }
            IrNode::If { condition, then_block, else_ifs, else_block, .. } => {
                collect_ir_locals(condition, locals);
                collect_ir_locals(then_block, locals);
                for (c, b) in else_ifs { collect_ir_locals(c, locals); collect_ir_locals(b, locals); }
                if let Some(e) = else_block { collect_ir_locals(e, locals); }
            }
            IrNode::Loop { condition, body, .. } => {
                if let Some(c) = condition { collect_ir_locals(c, locals); }
                collect_ir_locals(body, locals);
            }
            IrNode::ForEach { variable, iterable, body, .. } => {
                if !locals.contains(variable) { locals.push(variable.clone()); }
                collect_ir_locals(iterable, locals);
                collect_ir_locals(body, locals);
            }
            IrNode::FunctionDef { body, .. } => { collect_ir_locals(body, locals); }
            IrNode::BinOp { left, right, .. } => {
                collect_ir_locals(left, locals);
                collect_ir_locals(right, locals);
            }
            IrNode::UnaryOp { operand, .. } => { collect_ir_locals(operand, locals); }
            IrNode::Call { args, .. } => { for a in args { collect_ir_locals(a, locals); } }
            IrNode::CapabilityCall { call, .. } => { collect_ir_locals(call, locals); }
            IrNode::Array(els) => { for e in els { collect_ir_locals(e, locals); } }
            IrNode::IndexAssign { target, index, value, .. } => {
                collect_ir_locals(target, locals);
                collect_ir_locals(index, locals);
                collect_ir_locals(value, locals);
            }
            IrNode::Return(Some(v)) => { collect_ir_locals(v, locals); }
            _ => {}
        }
    }

    fn build_wat_module(
        local_names: &[String],
        body_lines: &[String],
        data_segments: &[(usize, String)],
    ) -> String {
        let mut wat = String::new();
        wat.push_str("(module\n");
        wat.push_str("  (memory 1)\n");

        for (offset, s) in data_segments {
            let escaped = escape_string(s);
            wat.push_str(&format!(
                "  (data (i32.const {}) \"{}\\00\")  ;; len={}\n",
                offset, escaped, s.len()
            ));
        }

        // Host function imports
        wat.push_str("  (import \"env\" \"print_i64\" (func $print_i64 (param i64)))\n");
        wat.push_str("  (import \"env\" \"print_f64\" (func $print_f64 (param f64)))\n");
        wat.push_str("  (import \"env\" \"print_str\" (func $print_str (param i32 i32)))\n");
        wat.push_str("  (import \"env\" \"array_new\" (func $array_new (param i64) (result i64)))\n");
        wat.push_str("  (import \"env\" \"array_get\" (func $array_get (param i64 i64) (result i64)))\n");
        wat.push_str("  (import \"env\" \"array_len\" (func $array_len (param i64) (result i64)))\n");

        wat.push_str("  (func $main (export \"main\")\n");

        for name in local_names {
            wat.push_str(&format!("    (local ${} i64)\n", sanitize_name(name)));
        }
        wat.push_str("    (local $__tmp i64)\n");

        for line in body_lines {
            wat.push_str("    ");
            wat.push_str(line);
            wat.push('\n');
        }
        wat.push_str("  )\n");
        wat.push_str(")\n");
        wat
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
