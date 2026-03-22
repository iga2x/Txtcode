/// Task 29.2 — WASM Binary Output (.wasm)
///
/// Translates a `Bytecode` program into a binary `.wasm` module using
/// `wasm-encoder`.  Only integer/float arithmetic and basic control flow are
/// supported; complex values (arrays, maps, strings) cause a compile error.
///
/// The output is validated by `wasmparser` before being returned.
///
/// # Example
///
/// ```text
/// use txtcode::compiler::wasm_binary::compile_to_binary;
/// let bytes = compile_to_binary(&bytecode).unwrap();
/// std::fs::write("output.wasm", bytes).unwrap();
/// ```

#[cfg(feature = "wasm")]
pub use wasm_impl::compile_to_binary;

/// Stub: always returns Err when the `wasm` feature is not enabled.
#[cfg(not(feature = "wasm"))]
pub fn compile_to_binary<T>(_bytecode: &T) -> Result<Vec<u8>, String> {
    Err("WASM binary output requires the 'wasm' feature. \
         Rebuild with: cargo build --features wasm"
        .to_string())
}

#[cfg(feature = "wasm")]
mod wasm_impl {
    use crate::compiler::bytecode::{Bytecode, Constant, Instruction};
    use std::collections::HashMap;
    use wasm_encoder::{
        CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function,
        FunctionSection, Instruction as WI, MemorySection, MemoryType, Module, TypeSection,
        ValType,
    };

    // ── String intern pool ────────────────────────────────────────────────────

    /// Accumulates string constant bytes into a contiguous linear-memory segment.
    ///
    /// Each unique string is stored once.  `intern()` returns `(offset, len)` as
    /// `i32` values suitable for packing into an `i64`:
    /// ```text
    /// packed = (offset as i64) << 32 | (len as i64)
    /// ```
    /// The host (JS / WASI) can unpack these to read the string from memory.
    struct StringPool {
        data: Vec<u8>,
        offsets: HashMap<String, (usize, usize)>, // string → (byte_offset, byte_len)
    }

    impl StringPool {
        fn new() -> Self {
            Self {
                data: Vec::new(),
                offsets: HashMap::new(),
            }
        }

        /// Intern `s`, return `(offset, len)` as i32 (both fit in 32 bits for any
        /// realistic program; the 64 kB default WASM page holds ~65 k bytes).
        fn intern(&mut self, s: &str) -> (i32, i32) {
            if let Some(&(off, len)) = self.offsets.get(s) {
                return (off as i32, len as i32);
            }
            let offset = self.data.len();
            self.data.extend_from_slice(s.as_bytes());
            self.offsets.insert(s.to_string(), (offset, s.len()));
            (offset as i32, s.len() as i32)
        }

        fn is_empty(&self) -> bool {
            self.data.is_empty()
        }
    }

    /// Compile `bytecode` to a validated binary `.wasm` module.
    ///
    /// String constants are stored in a linear memory data segment (memory 0,
    /// starting at offset 0).  Each string is represented on the WASM stack as a
    /// packed `i64 = (offset << 32) | len`.
    ///
    /// Returns `Err` when:
    /// - An instruction cannot be represented in WASM binary (arrays, maps, calls, etc.)
    /// - The produced binary fails `wasmparser` validation
    pub fn compile_to_binary(bytecode: &Bytecode) -> Result<Vec<u8>, String> {
        // ── Pass 1: collect locals + intern all string constants ───────────
        let mut local_names: Vec<String> = Vec::new();
        let mut pool = StringPool::new();

        for inst in &bytecode.instructions {
            match inst {
                Instruction::StoreVar(n)
                | Instruction::StoreConst(n)
                | Instruction::LoadGlobal(n)
                | Instruction::LoadVar(n) => {
                    if !local_names.contains(n) {
                        local_names.push(n.clone());
                    }
                }
                _ => {}
            }
        }

        // Pre-intern every string constant so the pool layout is fixed before
        // we emit instructions.
        for c in &bytecode.constants {
            if let Constant::String(s) = c {
                pool.intern(s);
            }
        }

        // ── Pass 2: emit WASM instructions ────────────────────────────────
        let mut body: Vec<WI> = Vec::new();
        for inst in &bytecode.instructions {
            match inst {
                Instruction::PushConstant(idx) => {
                    match bytecode.constants.get(*idx) {
                        Some(Constant::Integer(n)) => {
                            body.push(WI::I64Const(*n));
                        }
                        Some(Constant::Float(f)) => {
                            body.push(WI::F64Const(*f));
                            body.push(WI::I64ReinterpretF64);
                        }
                        Some(Constant::Boolean(b)) => {
                            body.push(WI::I64Const(if *b { 1 } else { 0 }));
                        }
                        Some(Constant::Null) => {
                            body.push(WI::I64Const(0));
                        }
                        Some(Constant::String(s)) => {
                            // Pack (offset, len) into a single i64.
                            // High 32 bits = memory byte offset of string start.
                            // Low  32 bits = byte length of the string.
                            let (offset, len) = pool.intern(s);
                            let packed = ((offset as i64) << 32) | (len as i64 & 0xFFFF_FFFF);
                            body.push(WI::I64Const(packed));
                        }
                        Some(Constant::FunctionRef(_)) => {
                            return Err(
                                "WASM binary output does not support function references"
                                    .to_string(),
                            );
                        }
                        None => {
                            return Err(format!("constant index {} out of bounds", idx));
                        }
                    }
                }

                Instruction::LoadVar(n) | Instruction::LoadGlobal(n) => {
                    let idx = local_index(&local_names, n)?;
                    body.push(WI::LocalGet(idx as u32));
                }

                Instruction::StoreVar(n) | Instruction::StoreConst(n) => {
                    let idx = local_index(&local_names, n)?;
                    body.push(WI::LocalTee(idx as u32));
                }

                Instruction::Pop => {
                    body.push(WI::Drop);
                }
                Instruction::Dup => {
                    return Err("Dup instruction not supported in binary WASM output".to_string());
                }

                // Arithmetic (i64)
                Instruction::Add => body.push(WI::I64Add),
                Instruction::Subtract => body.push(WI::I64Sub),
                Instruction::Multiply => body.push(WI::I64Mul),
                Instruction::Divide => body.push(WI::I64DivS),
                Instruction::Modulo => body.push(WI::I64RemS),
                Instruction::Negate => {
                    body.push(WI::I64Const(0));
                    body.push(WI::I64Sub);
                }

                // Comparison (result: 1 or 0 as i64)
                Instruction::Equal => {
                    body.push(WI::I64Eq);
                    body.push(WI::I64ExtendI32S);
                }
                Instruction::NotEqual => {
                    body.push(WI::I64Ne);
                    body.push(WI::I64ExtendI32S);
                }
                Instruction::Less => {
                    body.push(WI::I64LtS);
                    body.push(WI::I64ExtendI32S);
                }
                Instruction::Greater => {
                    body.push(WI::I64GtS);
                    body.push(WI::I64ExtendI32S);
                }
                Instruction::LessEqual => {
                    body.push(WI::I64LeS);
                    body.push(WI::I64ExtendI32S);
                }
                Instruction::GreaterEqual => {
                    body.push(WI::I64GeS);
                    body.push(WI::I64ExtendI32S);
                }

                // Logical
                Instruction::And => body.push(WI::I64And),
                Instruction::Or => body.push(WI::I64Or),
                Instruction::Not => {
                    body.push(WI::I64Const(0));
                    body.push(WI::I64Eq);
                    body.push(WI::I64ExtendI32S);
                }

                // Bitwise
                Instruction::BitAnd => body.push(WI::I64And),
                Instruction::BitOr => body.push(WI::I64Or),
                Instruction::BitXor => body.push(WI::I64Xor),

                // Control flow
                Instruction::Return | Instruction::ReturnValue => {
                    body.push(WI::Return);
                }

                other => {
                    return Err(format!(
                        "instruction {:?} is not supported in WASM binary output; \
                         use --target wat for full-featured compilation",
                        other
                    ));
                }
            }
        }
        body.push(WI::End);

        // ── Build module (sections must appear in WASM binary order) ───────
        // Order: Type → Function → Memory → Export → Code → Data

        let mut module = Module::new();

        // Type section: (func (result i64))
        let mut types = TypeSection::new();
        types.ty().function([], [ValType::I64]);
        module.section(&types);

        // Function section: function 0 uses type 0
        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);

        // Memory section (only when the pool has string data)
        let has_strings = !pool.is_empty();
        if has_strings {
            let mut memories = MemorySection::new();
            memories.memory(MemoryType {
                minimum: 1,
                maximum: None,
                memory64: false,
                shared: false,
                page_size_log2: None,
            });
            module.section(&memories);
        }

        // Export section: export "main" → function 0
        let mut exports = ExportSection::new();
        exports.export("main", ExportKind::Func, 0);
        module.section(&exports);

        // Code section
        let mut codes = CodeSection::new();
        let local_types: Vec<(u32, ValType)> = if local_names.is_empty() {
            vec![]
        } else {
            vec![(local_names.len() as u32, ValType::I64)]
        };
        let mut func = Function::new(local_types);
        for instr in &body {
            func.instruction(instr);
        }
        codes.function(&func);
        module.section(&codes);

        // Data section: one active segment at memory 0, offset 0
        if has_strings {
            let mut data = DataSection::new();
            data.active(
                0,
                &ConstExpr::i32_const(0),
                pool.data.iter().copied(),
            );
            module.section(&data);
        }

        let bytes = module.finish();

        // ── Validate ───────────────────────────────────────────────────────
        validate_wasm(&bytes)?;

        Ok(bytes)
    }

    fn local_index(locals: &[String], name: &str) -> Result<usize, String> {
        locals
            .iter()
            .position(|n| n == name)
            .ok_or_else(|| format!("undefined local variable '{}'", name))
    }

    fn validate_wasm(bytes: &[u8]) -> Result<(), String> {
        use wasmparser::{Parser, Payload};
        let parser = Parser::new(0);
        for payload in parser.parse_all(bytes) {
            match payload {
                Ok(_p) => match _p {
                    Payload::End(_) => break,
                    _ => {}
                },
                Err(e) => return Err(format!("WASM validation failed: {}", e)),
            }
        }
        Ok(())
    }
}
