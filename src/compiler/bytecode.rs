use crate::parser::ast::*;
use serde::{Deserialize, Serialize};

/// Bytecode representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bytecode {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,
    /// Maps instruction index → source line number (1-based).
    /// Not every instruction has an entry; use a linear scan to find the nearest.
    pub debug_info: Vec<(usize, usize)>,
}

/// Constant pool for bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    /// A bytecode function reference by name — distinct from Constant::String
    /// so the VM can distinguish lambdas from plain string values.
    FunctionRef(String),
}

impl Constant {
    /// Convert to a runtime `Value` for use in the WASM compiler.
    pub fn to_value(&self) -> crate::runtime::Value {
        match self {
            Constant::Integer(n) => crate::runtime::Value::Integer(*n),
            Constant::Float(f) => crate::runtime::Value::Float(*f),
            Constant::String(s) => crate::runtime::Value::String(s.as_str().into()),
            Constant::Boolean(b) => crate::runtime::Value::Boolean(*b),
            Constant::Null => crate::runtime::Value::Null,
            Constant::FunctionRef(s) => crate::runtime::Value::String(s.as_str().into()),
        }
    }
}

impl std::hash::Hash for Constant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Constant::Integer(i) => {
                state.write_u8(0);
                i.hash(state);
            }
            Constant::Float(f) => {
                state.write_u8(1);
                // Hash float as bits to avoid NaN issues
                f.to_bits().hash(state);
            }
            Constant::String(s) => {
                state.write_u8(2);
                s.hash(state);
            }
            Constant::Boolean(b) => {
                state.write_u8(3);
                b.hash(state);
            }
            Constant::Null => {
                state.write_u8(4);
            }
            Constant::FunctionRef(s) => {
                state.write_u8(5);
                s.hash(state);
            }
        }
    }
}

impl std::cmp::Eq for Constant {}

impl std::cmp::PartialEq for Constant {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Constant::Integer(a), Constant::Integer(b)) => a == b,
            (Constant::Float(a), Constant::Float(b)) => {
                // Handle NaN and infinity
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            (Constant::String(a), Constant::String(b)) => a == b,
            (Constant::Boolean(a), Constant::Boolean(b)) => a == b,
            (Constant::Null, Constant::Null) => true,
            (Constant::FunctionRef(a), Constant::FunctionRef(b)) => a == b,
            _ => false,
        }
    }
}

/// Bytecode instructions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Instruction {
    // Stack operations
    PushConstant(usize), // Push constant from constant pool
    Pop,                 // Pop top of stack
    Dup,                 // Duplicate top of stack

    // Variable operations
    LoadVar(String),    // Load variable onto stack
    StoreVar(String),   // Store top of stack to variable
    StoreConst(String), // Store top of stack to immutable constant variable
    LoadGlobal(String), // Load global variable

    // Arithmetic operations
    Add,      // Pop two values, push sum
    Subtract, // Pop two values, push difference
    Multiply, // Pop two values, push product
    Divide,   // Pop two values, push quotient
    Modulo,   // Pop two values, push remainder
    Power,    // Pop two values, push power
    Negate,   // Pop value, push negated

    // Comparison operations
    Equal,        // Pop two values, push equality
    NotEqual,     // Pop two values, push inequality
    Less,         // Pop two values, push less than
    Greater,      // Pop two values, push greater than
    LessEqual,    // Pop two values, push less or equal
    GreaterEqual, // Pop two values, push greater or equal

    // Logical operations
    And, // Pop two values, push AND
    Or,  // Pop two values, push OR
    Not, // Pop value, push NOT

    // Bitwise operations
    BitAnd,     // Pop two values, push bitwise AND
    BitOr,      // Pop two values, push bitwise OR
    BitXor,     // Pop two values, push bitwise XOR
    LeftShift,  // Pop two values, push left shift
    RightShift, // Pop two values, push right shift
    BitNot,     // Pop value, push bitwise NOT

    // Control flow
    Jump(usize),        // Unconditional jump to instruction index
    JumpIfFalse(usize), // Pop value, jump if false
    JumpIfTrue(usize),  // Pop value, jump if true

    // Function operations
    Call(String, usize), // Call function with name and arg count
    Return,              // Return from function
    ReturnValue,         // Pop value and return it
    /// Register a user-defined function: name, param names, body start IP.
    /// Emitted after a jump-around so normal execution skips the body.
    RegisterFunction(String, Vec<String>, usize),

    // Array/Map operations
    BuildArray(usize), // Pop N values, build array
    BuildMap(usize),   // Pop 2N values (key-value pairs), build map
    Index,             // Pop index/key and object, push indexed value
    SetIndex,          // Pop value, index/key, and object, set indexed value

    // Object operations
    GetField(String), // Pop object, push field value
    SetField(String), // Pop value and object, set field

    // Type operations
    TypeOf, // Pop value, push type string

    // Null-safe operations
    /// Pop two values (value, default); push value if not null, else push default.
    /// Right-hand side is always evaluated (no short-circuit at bytecode level).
    NullCoalesce,

    // Optional chaining — fully implemented in both compiler and bytecode VM.
    /// Optional member access: pop object, push `obj.field` or `null` if object is null.
    OptionalGetField(String),
    /// Optional index: pop index and object, push `obj[idx]` or `null` if object is null.
    OptionalIndex,
    /// Optional call: call function or return `null` if target is null.
    OptionalCall(String, usize),

    // Control flow helpers
    Label(usize), // Label for jump targets

    // For-loop iterator support
    /// Pop iterable from stack, set up iterator for `var`. If iterable is empty, jump to `usize`.
    ForSetup(String, usize),
    /// Advance the top for-iterator. If more items remain, store next in var and jump to `usize`
    /// (loop body start). If exhausted, pop the iterator and fall through.
    ForNext(usize),
    /// Pop the top for-iterator (used when break exits a for loop early).
    ForCleanup,

    // Collection operations
    /// Pop N values from stack and build a Set (deduplicated).
    BuildSet(usize),

    // Method dispatch
    /// Pop object, pop N args (args pushed before object); call method; push result.
    CallMethod(String, usize),

    // Slice operation
    /// Stack before: [target, start, end, step] with step on top (Null if omitted).
    /// Push sliced array or string.
    Slice,

    /// Import a module by path, executing it and merging its exported names into scope.
    ImportModule(String),

    // No operation
    Nop,

    // Try-catch support
    /// Set up a catch handler. catch_ip = target IP on error, finally_ip = optional finally start.
    /// error_var = optional variable name to store the error message string.
    SetupCatch(usize, Option<usize>, Option<String>),
    /// Remove the most recently pushed catch handler (after try body succeeds).
    PopCatch,
    /// Throw: pop top of stack and raise as RuntimeError.
    Throw,

    // Result type construction
    /// Build Ok(value): pop value from stack, push Result(true, value)
    BuildOk,
    /// Build Err(value): pop value from stack, push Result(false, value)
    BuildErr,
    /// `?` error propagation: pop Result; if Err → early-return Err; if Ok → push unwrapped value
    Propagate,

    // Struct literal construction
    /// Pop struct_name (string) + N * (key, value) pairs from stack, build Struct value.
    /// Stack layout: struct_name, key0, val0, key1, val1, ... (struct_name first pushed)
    BuildStructLiteral(usize), // N = number of fields

    // Spread support
    /// Pop a value and append it to the array currently on top of stack.
    ArrayAppend,
    /// Pop an array and extend the array currently on top of stack with its elements.
    ArrayExtend,

    /// Pipe: stack has [arg, func]. Pop func, pop arg, call func(arg), push result.
    /// Used when |> rhs is a complex expression (not a simple identifier desugared at parse time).
    Pipe,

    /// Await a Future: pop Value::Future from stack, block until resolved, push result.
    /// `await` on a non-Future value is a transparent no-op (JavaScript semantics).
    Await,
}

/// Per-loop tracking for break/continue jump patching.
struct LoopContext {
    /// Index of the instruction to jump to on `continue`.
    /// Known immediately for while/do-while; filled in after ForNext is emitted for for-loops.
    loop_start: Option<usize>,
    /// Whether this loop is a `for … in` loop (needs ForCleanup on break).
    is_for_loop: bool,
    /// Instruction indices that hold a `Jump(0)` placeholder for `break`.
    break_patches: Vec<usize>,
    /// Instruction indices that hold a `Jump(0)` placeholder for `continue` inside for-loops
    /// (where loop_start is not yet known when the body is being compiled).
    continue_patches: Vec<usize>,
}

/// Bytecode compiler
pub struct BytecodeCompiler {
    constants: Vec<Constant>,
    constant_map: std::collections::HashMap<Constant, usize>,
    loop_context: Vec<LoopContext>,
    debug_info: Vec<(usize, usize)>,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            constant_map: std::collections::HashMap::new(),
            loop_context: Vec::new(),
            debug_info: Vec::new(),
        }
    }

    /// Compile a program to bytecode
    pub fn compile(&mut self, program: &Program) -> Bytecode {
        let mut instructions = Vec::new();

        for statement in &program.statements {
            self.compile_statement(statement, &mut instructions);
        }

        Bytecode {
            instructions,
            constants: self.constants.clone(),
            debug_info: self.debug_info.clone(),
        }
    }

    /// Record the source line for the next instruction to be emitted.
    fn record_line(&mut self, instructions: &[Instruction], line: usize) {
        if line > 0 {
            let ip = instructions.len();
            // Only push if this is a new ip or a different line than the last entry
            if self.debug_info.last().is_none_or(|&(i, l)| i != ip || l != line) {
                self.debug_info.push((ip, line));
            }
        }
    }

    fn compile_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>) {
        // Record source line before compiling each statement
        if let Some((line, _)) = stmt.source_location() {
            self.record_line(instructions, line);
        }
        match stmt {
            Statement::Assignment { pattern, value, .. } => {
                self.compile_expression(value, instructions);
                match pattern {
                    Pattern::Identifier(name) => {
                        instructions.push(Instruction::StoreVar(name.clone()));
                    }
                    _ => {
                        instructions.push(Instruction::Pop);
                    }
                }
            }
            Statement::FunctionDef {
                name, params, body, ..
            } => {
                // Jump over function body (patched below)
                let jump_idx = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: Jump(after_body)

                let body_start = instructions.len();
                for body_stmt in body {
                    self.compile_statement(body_stmt, instructions);
                }
                // Ensure function ends with a return
                match instructions.last() {
                    Some(Instruction::Return) | Some(Instruction::ReturnValue) => {}
                    _ => instructions.push(Instruction::Return),
                }

                let after_body = instructions.len();
                instructions[jump_idx] = Instruction::Jump(after_body);

                // Extract positional param names
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                // Emit registration instruction (runs at definition time)
                instructions.push(Instruction::RegisterFunction(
                    name.clone(),
                    param_names,
                    body_start,
                ));
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expression(expr, instructions);
                    instructions.push(Instruction::ReturnValue);
                } else {
                    instructions.push(Instruction::Return);
                }
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr, instructions);
                instructions.push(Instruction::Pop);
            }
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                // Compile: if cond { then } else if c2 { b2 } ... else { els }
                // Desugared into nested JumpIfFalse chains.
                let mut end_patches: Vec<usize> = Vec::new();

                // First branch
                self.compile_expression(condition, instructions);
                let skip_then = instructions.len();
                instructions.push(Instruction::Nop); // JumpIfFalse(next)
                for stmt in then_branch {
                    self.compile_statement(stmt, instructions);
                }
                let skip_rest = instructions.len();
                instructions.push(Instruction::Nop); // Jump(end)
                end_patches.push(skip_rest);
                let next_idx = instructions.len();
                instructions[skip_then] = Instruction::JumpIfFalse(next_idx);

                // else-if branches
                for (ei_cond, ei_body) in else_if_branches {
                    self.compile_expression(ei_cond, instructions);
                    let skip_ei = instructions.len();
                    instructions.push(Instruction::Nop); // JumpIfFalse(next)
                    for stmt in ei_body {
                        self.compile_statement(stmt, instructions);
                    }
                    let skip_rest2 = instructions.len();
                    instructions.push(Instruction::Nop); // Jump(end)
                    end_patches.push(skip_rest2);
                    let next_ei = instructions.len();
                    instructions[skip_ei] = Instruction::JumpIfFalse(next_ei);
                }

                // else branch
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.compile_statement(stmt, instructions);
                    }
                }

                let end_idx = instructions.len();
                for ep in end_patches {
                    instructions[ep] = Instruction::Jump(end_idx);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                let loop_start = instructions.len();

                self.loop_context.push(LoopContext {
                    loop_start: Some(loop_start),
                    is_for_loop: false,
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });

                self.compile_expression(condition, instructions);
                let exit_patch = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: JumpIfFalse(end)

                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }
                instructions.push(Instruction::Jump(loop_start)); // back-jump

                let end_idx = instructions.len();
                instructions[exit_patch] = Instruction::JumpIfFalse(end_idx);

                // Patch break jumps
                let ctx = match self.loop_context.pop() {
                    Some(c) => c,
                    None => {
                        // Should never happen — validator catches break/continue outside loops.
                        // Emit nothing and return to avoid a panic in case the AST is malformed.
                        eprintln!("Internal compiler error: loop context stack underflow (break/continue outside loop)");
                        return;
                    }
                };
                for bp in ctx.break_patches {
                    instructions[bp] = Instruction::Jump(end_idx);
                }
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                let loop_start = instructions.len();

                self.loop_context.push(LoopContext {
                    loop_start: Some(loop_start),
                    is_for_loop: false,
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });

                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }

                // Continue jumps here (to condition check)
                let condition_start = instructions.len();
                // Patch continue_patches for do-while (loop_start was set to body start, but
                // continue should re-evaluate the condition — set to condition_start)
                self.compile_expression(condition, instructions);
                let jit_idx = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: JumpIfTrue(loop_start)
                instructions[jit_idx] = Instruction::JumpIfTrue(loop_start);

                let end_idx = instructions.len();

                let ctx = match self.loop_context.pop() {
                    Some(c) => c,
                    None => {
                        // Should never happen — validator catches break/continue outside loops.
                        // Emit nothing and return to avoid a panic in case the AST is malformed.
                        eprintln!("Internal compiler error: loop context stack underflow (break/continue outside loop)");
                        return;
                    }
                };
                for bp in ctx.break_patches {
                    instructions[bp] = Instruction::Jump(end_idx);
                }
                for cp in ctx.continue_patches {
                    instructions[cp] = Instruction::Jump(condition_start);
                }
                // Suppress unused warning
                let _ = condition_start;
            }
            Statement::For {
                variable,
                iterable,
                body,
                ..
            } => {
                self.compile_expression(iterable, instructions);

                // ForSetup: pop array, if empty jump to end (placeholder)
                let setup_idx = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: ForSetup(var, end)

                let body_start = instructions.len();

                self.loop_context.push(LoopContext {
                    loop_start: None, // ForNext index not yet known
                    is_for_loop: true,
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });

                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }

                // ForNext: advance iterator; if more, jump to body_start; else fall through
                let for_next_idx = instructions.len();
                instructions.push(Instruction::ForNext(body_start));

                let end_idx = instructions.len();

                // Patch ForSetup placeholder
                instructions[setup_idx] = Instruction::ForSetup(variable.clone(), end_idx);

                let ctx = match self.loop_context.pop() {
                    Some(c) => c,
                    None => {
                        // Should never happen — validator catches break/continue outside loops.
                        // Emit nothing and return to avoid a panic in case the AST is malformed.
                        eprintln!("Internal compiler error: loop context stack underflow (break/continue outside loop)");
                        return;
                    }
                };
                // Patch break: emit ForCleanup before each break jump
                for bp in ctx.break_patches {
                    // bp is the ForCleanup instruction; bp+1 is the Jump placeholder
                    instructions[bp + 1] = Instruction::Jump(end_idx);
                }
                // Patch continue: jump to ForNext
                for cp in ctx.continue_patches {
                    instructions[cp] = Instruction::Jump(for_next_idx);
                }
            }
            Statement::Repeat { count, body, .. } => {
                // Emit: counter = 0; while counter < count { body; counter++ }
                // We use a hidden var name unlikely to clash
                let counter_var = "__repeat_counter__".to_string();
                let count_var = "__repeat_limit__".to_string();

                // store counter = 0
                let zero_idx = self.add_constant(&Literal::Integer(0));
                instructions.push(Instruction::PushConstant(zero_idx));
                instructions.push(Instruction::StoreVar(counter_var.clone()));

                // store limit = count expression
                self.compile_expression(count, instructions);
                instructions.push(Instruction::StoreVar(count_var.clone()));

                let loop_start = instructions.len();
                self.loop_context.push(LoopContext {
                    loop_start: Some(loop_start),
                    is_for_loop: false,
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });

                // while counter < limit
                instructions.push(Instruction::LoadVar(counter_var.clone()));
                instructions.push(Instruction::LoadVar(count_var.clone()));
                instructions.push(Instruction::Less);
                let exit_patch = instructions.len();
                instructions.push(Instruction::Nop);

                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }

                // counter++
                let one_idx = self.add_constant(&Literal::Integer(1));
                instructions.push(Instruction::LoadVar(counter_var.clone()));
                instructions.push(Instruction::PushConstant(one_idx));
                instructions.push(Instruction::Add);
                instructions.push(Instruction::StoreVar(counter_var.clone()));

                instructions.push(Instruction::Jump(loop_start));

                let end_idx = instructions.len();
                instructions[exit_patch] = Instruction::JumpIfFalse(end_idx);

                let ctx = match self.loop_context.pop() {
                    Some(c) => c,
                    None => {
                        // Should never happen — validator catches break/continue outside loops.
                        // Emit nothing and return to avoid a panic in case the AST is malformed.
                        eprintln!("Internal compiler error: loop context stack underflow (break/continue outside loop)");
                        return;
                    }
                };
                for bp in ctx.break_patches {
                    instructions[bp] = Instruction::Jump(end_idx);
                }
            }
            Statement::Const { name, value, .. } => {
                self.compile_expression(value, instructions);
                instructions.push(Instruction::StoreConst(name.clone()));
            }
            Statement::Match {
                value,
                cases,
                default,
                ..
            } => {
                // Compile: evaluate match value once, store in hidden var, compare each case.
                // Each case compiles to: load hidden, compare with pattern constant, JumpIfFalse(next)
                self.compile_expression(value, instructions);
                let match_var = "__match__".to_string();
                instructions.push(Instruction::StoreVar(match_var.clone()));

                let mut end_patches: Vec<usize> = Vec::new();

                for (pattern, guard, body) in cases {
                    // Check if this is a wildcard pattern (_)
                    let is_wildcard = matches!(pattern, crate::parser::ast::Pattern::Ignore);

                    let mut case_fail_patch: Option<usize> = None;

                    if !is_wildcard {
                        // Load match value and push pattern value
                        instructions.push(Instruction::LoadVar(match_var.clone()));

                        // Compile pattern as a constant expression if possible
                        match pattern {
                            crate::parser::ast::Pattern::Identifier(name) => {
                                // Patterns stored as __literal_<value> (see parser/patterns.rs)
                                if let Some(raw) = name.strip_prefix("__literal_") {
                                    // Try integer
                                    if let Ok(i) = raw.parse::<i64>() {
                                        let idx = self.add_constant(&Literal::Integer(i));
                                        instructions.push(Instruction::PushConstant(idx));
                                    } else if let Ok(f) = raw.parse::<f64>() {
                                        let idx = self.add_constant(&Literal::Float(f));
                                        instructions.push(Instruction::PushConstant(idx));
                                    } else if raw == "true" {
                                        let idx = self.add_constant(&Literal::Boolean(true));
                                        instructions.push(Instruction::PushConstant(idx));
                                    } else if raw == "false" {
                                        let idx = self.add_constant(&Literal::Boolean(false));
                                        instructions.push(Instruction::PushConstant(idx));
                                    } else if raw == "null" {
                                        let idx = self.add_constant(&Literal::Null);
                                        instructions.push(Instruction::PushConstant(idx));
                                    } else {
                                        // String literal (value as-is)
                                        let idx =
                                            self.add_constant(&Literal::String(raw.to_string()));
                                        instructions.push(Instruction::PushConstant(idx));
                                    }
                                } else {
                                    // Regular identifier — load as variable
                                    instructions.push(Instruction::LoadVar(name.clone()));
                                }
                            }
                            crate::parser::ast::Pattern::Array(sub_pats) => {
                                // Array pattern: [a, b, c] — check match value is array
                                // then bind each element to the corresponding variable name.
                                // match value is already on stack (LoadVar match_var).
                                // For each sub-pattern that is a simple identifier, emit:
                                //   LoadVar __match__; PushConstant(index); Index; StoreVar name
                                let fail_patch = instructions.len();
                                instructions.push(Instruction::Nop); // JumpIfFalse(next) if not array (type check not done; treated as wildcard match then destructure)
                                                                     // Pop the loaded match value - we'll re-load for each element
                                instructions.push(Instruction::Pop);
                                let _ = case_fail_patch.take(); // handled inline
                                                                // Destructure: bind each indexed element
                                for (idx, sub_pat) in sub_pats.iter().enumerate() {
                                    if let crate::parser::ast::Pattern::Identifier(var_name) =
                                        sub_pat
                                    {
                                        if !var_name.starts_with("__literal_") && var_name != "_" {
                                            let idx_const =
                                                self.add_constant(&Literal::Integer(idx as i64));
                                            instructions
                                                .push(Instruction::LoadVar(match_var.clone()));
                                            instructions.push(Instruction::PushConstant(idx_const));
                                            instructions.push(Instruction::Index);
                                            instructions
                                                .push(Instruction::StoreVar(var_name.clone()));
                                        }
                                    }
                                }
                                // Guard check (if any)
                                if let Some(g) = guard {
                                    self.compile_expression(g, instructions);
                                    let gfail = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    for s in body {
                                        self.compile_statement(s, instructions);
                                    }
                                    let ep = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    end_patches.push(ep);
                                    let next = instructions.len();
                                    instructions[fail_patch] = Instruction::JumpIfFalse(next);
                                    instructions[gfail] = Instruction::JumpIfFalse(next);
                                } else {
                                    for s in body {
                                        self.compile_statement(s, instructions);
                                    }
                                    let ep = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    end_patches.push(ep);
                                    instructions[fail_patch] =
                                        Instruction::JumpIfFalse(instructions.len());
                                }
                                continue;
                            }
                            crate::parser::ast::Pattern::Struct { fields, .. } => {
                                // Struct/map pattern: {x, y} — treat match value as map, bind fields.
                                // match value is already on stack from LoadVar.
                                instructions.push(Instruction::Pop); // remove the match value loaded above
                                for (field_name, sub_pat) in fields {
                                    let var_name = match sub_pat {
                                        crate::parser::ast::Pattern::Identifier(n) if n != "_" => {
                                            n.clone()
                                        }
                                        _ => field_name.clone(),
                                    };
                                    let key_idx =
                                        self.add_constant(&Literal::String(field_name.clone()));
                                    instructions.push(Instruction::LoadVar(match_var.clone()));
                                    instructions.push(Instruction::PushConstant(key_idx));
                                    instructions.push(Instruction::Index);
                                    instructions.push(Instruction::StoreVar(var_name));
                                }
                                if let Some(g) = guard {
                                    self.compile_expression(g, instructions);
                                    let gfail = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    for s in body {
                                        self.compile_statement(s, instructions);
                                    }
                                    let ep = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    end_patches.push(ep);
                                    instructions[gfail] =
                                        Instruction::JumpIfFalse(instructions.len());
                                } else {
                                    for s in body {
                                        self.compile_statement(s, instructions);
                                    }
                                    let ep = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    end_patches.push(ep);
                                }
                                continue;
                            }
                            // Task 12.5: Or-pattern — `1 | 2 | 3` matches any listed value
                            crate::parser::ast::Pattern::Or(sub_pats) => {
                                // Outer loop loaded LoadVar(match_var) — pop it; re-load per compare
                                instructions.push(Instruction::Pop);
                                let mut first_or = true;
                                for sub_pat in sub_pats {
                                    if let crate::parser::ast::Pattern::Identifier(name) = sub_pat {
                                        instructions.push(Instruction::LoadVar(match_var.clone()));
                                        if let Some(raw) = name.strip_prefix("__literal_") {
                                            if let Ok(i) = raw.parse::<i64>() {
                                                let idx = self.add_constant(&Literal::Integer(i));
                                                instructions.push(Instruction::PushConstant(idx));
                                            } else if let Ok(f) = raw.parse::<f64>() {
                                                let idx = self.add_constant(&Literal::Float(f));
                                                instructions.push(Instruction::PushConstant(idx));
                                            } else {
                                                let idx = self.add_constant(&Literal::String(raw.to_string()));
                                                instructions.push(Instruction::PushConstant(idx));
                                            }
                                        } else {
                                            instructions.push(Instruction::LoadVar(name.clone()));
                                        }
                                        instructions.push(Instruction::Equal);
                                        if !first_or {
                                            instructions.push(Instruction::Or);
                                        }
                                        first_or = false;
                                    }
                                }
                                // Stack: [bool] — JumpIfFalse skips body if false
                                case_fail_patch = Some(instructions.len());
                                instructions.push(Instruction::Nop); // JumpIfFalse(next_case)
                            }
                            // Task 12.5: Range pattern — `1..=5` matches val in [start, end]
                            crate::parser::ast::Pattern::Range(start_expr, end_expr) => {
                                // Outer loop loaded LoadVar(match_var) — pop it; re-load per compare
                                instructions.push(Instruction::Pop);
                                // val >= start
                                instructions.push(Instruction::LoadVar(match_var.clone()));
                                self.compile_expression(start_expr, instructions);
                                instructions.push(Instruction::GreaterEqual);
                                // val <= end
                                instructions.push(Instruction::LoadVar(match_var.clone()));
                                self.compile_expression(end_expr, instructions);
                                instructions.push(Instruction::LessEqual);
                                // Both must be true
                                instructions.push(Instruction::And);
                                // Stack: [bool] — JumpIfFalse skips body if false
                                case_fail_patch = Some(instructions.len());
                                instructions.push(Instruction::Nop); // JumpIfFalse(next_case)
                            }
                            _ => {
                                // Unknown/unimplemented pattern — treat as wildcard (always matches)
                                instructions.push(Instruction::Pop); // pop match value loaded above
                                if let Some(g) = guard {
                                    self.compile_expression(g, instructions);
                                    let gp = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    for s in body {
                                        self.compile_statement(s, instructions);
                                    }
                                    let ep = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    end_patches.push(ep);
                                    instructions[gp] = Instruction::JumpIfFalse(ep + 1);
                                } else {
                                    for s in body {
                                        self.compile_statement(s, instructions);
                                    }
                                    let ep = instructions.len();
                                    instructions.push(Instruction::Nop);
                                    end_patches.push(ep);
                                }
                                continue;
                            }
                        }
                        // For Or/Range patterns, case_fail_patch is already set inside the branch
                        // For regular Identifier patterns, we still need Equal + JumpIfFalse
                        if !matches!(pattern, crate::parser::ast::Pattern::Or(_) | crate::parser::ast::Pattern::Range(..)) {
                            instructions.push(Instruction::Equal);
                            case_fail_patch = Some(instructions.len());
                            instructions.push(Instruction::Nop); // JumpIfFalse(next_case)
                        }
                    }

                    // Optional guard
                    if let Some(g) = guard {
                        self.compile_expression(g, instructions);
                        let gfail = instructions.len();
                        instructions.push(Instruction::Nop);
                        // If guard fails, also skip to next case
                        if let Some(cfp) = case_fail_patch {
                            for s in body {
                                self.compile_statement(s, instructions);
                            }
                            let ep = instructions.len();
                            instructions.push(Instruction::Nop);
                            end_patches.push(ep);
                            let next = instructions.len();
                            instructions[cfp] = Instruction::JumpIfFalse(next);
                            instructions[gfail] = Instruction::JumpIfFalse(next);
                        } else {
                            for s in body {
                                self.compile_statement(s, instructions);
                            }
                            let ep = instructions.len();
                            instructions.push(Instruction::Nop);
                            end_patches.push(ep);
                            instructions[gfail] = Instruction::JumpIfFalse(instructions.len());
                        }
                        continue;
                    }

                    // Compile case body
                    for s in body {
                        self.compile_statement(s, instructions);
                    }
                    let ep = instructions.len();
                    instructions.push(Instruction::Nop); // Jump(end)
                    end_patches.push(ep);

                    // Patch case-fail jump to after this case's body+jump
                    if let Some(cfp) = case_fail_patch {
                        instructions[cfp] = Instruction::JumpIfFalse(instructions.len());
                    }
                }

                // Compile default branch
                if let Some(default_body) = default {
                    for s in default_body {
                        self.compile_statement(s, instructions);
                    }
                }

                let end_idx = instructions.len();
                for ep in end_patches {
                    instructions[ep] = Instruction::Jump(end_idx);
                }
            }
            Statement::Try {
                body,
                catch,
                finally,
                ..
            } => {
                // Real try-catch with error interception via SetupCatch/PopCatch.
                // Layout:
                //   SetupCatch(catch_ip, finally_ip, err_var)
                //   <try body>
                //   PopCatch
                //   Jump(after_catch)
                // catch_ip:
                //   <catch body>
                //   Jump(finally_ip or end)
                // finally_ip:
                //   <finally body>
                // end:

                let err_var = catch.as_ref().map(|(v, _)| v.clone());

                // SetupCatch placeholder (catch_ip filled in after try body)
                let setup_idx = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: SetupCatch(catch_ip, finally_ip, err_var)

                // Compile try body
                for s in body {
                    self.compile_statement(s, instructions);
                }

                // PopCatch (success path)
                instructions.push(Instruction::PopCatch);

                // Jump over catch body (success path)
                let jump_over_catch = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: Jump(after_catch_or_finally)

                let catch_ip = instructions.len();

                // Compile catch body
                if let Some((_, catch_body)) = catch {
                    for s in catch_body {
                        self.compile_statement(s, instructions);
                    }
                }

                // Jump to finally (or end)
                let jump_to_finally = instructions.len();
                instructions.push(Instruction::Nop); // placeholder: Jump(finally_or_end)

                let finally_ip = instructions.len();

                // Compile finally body
                if let Some(finally_body) = finally {
                    for s in finally_body {
                        self.compile_statement(s, instructions);
                    }
                }

                // Patch placeholders
                // Both success and catch paths jump to finally_ip;
                // if no finally body, finally_ip == end_idx so this is correct.
                let finally_ip_opt = if finally.is_some() {
                    Some(finally_ip)
                } else {
                    None
                };
                instructions[setup_idx] =
                    Instruction::SetupCatch(catch_ip, finally_ip_opt, err_var);
                instructions[jump_over_catch] = Instruction::Jump(finally_ip);
                instructions[jump_to_finally] = Instruction::Jump(finally_ip);
            }
            Statement::Break { .. } => {
                if let Some(ctx) = self.loop_context.last_mut() {
                    if ctx.is_for_loop {
                        // Cleanup for-loop iterator before breaking
                        instructions.push(Instruction::ForCleanup);
                        let jump_idx = instructions.len();
                        instructions.push(Instruction::Nop); // placeholder Jump(end)
                        ctx.break_patches.push(jump_idx - 1); // ForCleanup idx; patch is at jump_idx
                                                              // Actually record jump_idx as the placeholder:
                        ctx.break_patches.pop();
                        ctx.break_patches.push(jump_idx);
                    } else {
                        let jump_idx = instructions.len();
                        instructions.push(Instruction::Nop); // placeholder Jump(end)
                        ctx.break_patches.push(jump_idx);
                    }
                } else {
                    instructions.push(Instruction::Nop); // break outside loop = no-op
                }
            }
            Statement::Continue { .. } => {
                if let Some(ctx) = self.loop_context.last_mut() {
                    if let Some(loop_start) = ctx.loop_start {
                        // While/do-while: jump directly to loop_start
                        instructions.push(Instruction::Jump(loop_start));
                    } else {
                        // For loop: loop_start (ForNext) not known yet — use placeholder
                        let jump_idx = instructions.len();
                        instructions.push(Instruction::Nop);
                        ctx.continue_patches.push(jump_idx);
                    }
                } else {
                    instructions.push(Instruction::Nop); // continue outside loop = no-op
                }
            }
            Statement::IndexAssignment {
                target,
                index,
                value,
                ..
            } => {
                // Stack: push new_value, push index, push target_obj, SetIndex, store back
                self.compile_expression(value, instructions);
                self.compile_expression(index, instructions);
                match target {
                    Expression::Identifier(name) => {
                        instructions.push(Instruction::LoadVar(name.clone()));
                        instructions.push(Instruction::SetIndex);
                        instructions.push(Instruction::StoreVar(name.clone()));
                    }
                    _ => {
                        // Complex chained target: emit Nop and discard
                        instructions.push(Instruction::Pop); // discard value
                        instructions.push(Instruction::Pop); // discard index
                    }
                }
            }
            Statement::CompoundAssignment {
                name, op, value, ..
            } => {
                instructions.push(Instruction::LoadVar(name.clone()));
                self.compile_expression(value, instructions);
                instructions.push(match op {
                    BinaryOperator::Add => Instruction::Add,
                    BinaryOperator::Subtract => Instruction::Subtract,
                    BinaryOperator::Multiply => Instruction::Multiply,
                    BinaryOperator::Divide => Instruction::Divide,
                    BinaryOperator::Modulo => Instruction::Modulo,
                    BinaryOperator::Power => Instruction::Power,
                    BinaryOperator::BitwiseAnd => Instruction::BitAnd,
                    BinaryOperator::BitwiseOr => Instruction::BitOr,
                    BinaryOperator::BitwiseXor => Instruction::BitXor,
                    BinaryOperator::LeftShift => Instruction::LeftShift,
                    BinaryOperator::RightShift => Instruction::RightShift,
                    _ => Instruction::Nop,
                });
                instructions.push(Instruction::StoreVar(name.clone()));
            }
            Statement::TypeAlias { .. } => {
                // Type aliases are no-ops in bytecode (no runtime effect)
                instructions.push(Instruction::Nop);
            }
            Statement::NamedError { name, message, .. } => {
                // Named error: evaluate message and store in __named_error_<name>
                self.compile_expression(message, instructions);
                instructions.push(Instruction::StoreVar(format!("__named_error_{}", name)));
            }
            Statement::Import { modules, from, .. } => {
                // Emit an ImportModule instruction for each module name.
                // If `from` is Some("math"), we import "math".
                // If modules = ["math"], we import "math".
                if let Some(source) = from {
                    instructions.push(Instruction::ImportModule(source.clone()));
                } else {
                    for module in modules {
                        instructions.push(Instruction::ImportModule(module.clone()));
                    }
                }
            }
            _ => {
                instructions.push(Instruction::Nop);
            }
        }
    }

    fn compile_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) {
        match expr {
            Expression::Literal(lit) => {
                let const_idx = self.add_constant(lit);
                instructions.push(Instruction::PushConstant(const_idx));
            }
            Expression::Identifier(name) => {
                instructions.push(Instruction::LoadVar(name.clone()));
            }
            Expression::BinaryOp {
                left, op, right, ..
            } => {
                self.compile_expression(left, instructions);
                self.compile_expression(right, instructions);
                instructions.push(match op {
                    BinaryOperator::Add => Instruction::Add,
                    BinaryOperator::Subtract => Instruction::Subtract,
                    BinaryOperator::Multiply => Instruction::Multiply,
                    BinaryOperator::Divide => Instruction::Divide,
                    BinaryOperator::Modulo => Instruction::Modulo,
                    BinaryOperator::Power => Instruction::Power,
                    BinaryOperator::Equal => Instruction::Equal,
                    BinaryOperator::NotEqual => Instruction::NotEqual,
                    BinaryOperator::Less => Instruction::Less,
                    BinaryOperator::Greater => Instruction::Greater,
                    BinaryOperator::LessEqual => Instruction::LessEqual,
                    BinaryOperator::GreaterEqual => Instruction::GreaterEqual,
                    BinaryOperator::And => Instruction::And,
                    BinaryOperator::Or => Instruction::Or,
                    BinaryOperator::BitwiseAnd => Instruction::BitAnd,
                    BinaryOperator::BitwiseOr => Instruction::BitOr,
                    BinaryOperator::BitwiseXor => Instruction::BitXor,
                    BinaryOperator::LeftShift => Instruction::LeftShift,
                    BinaryOperator::RightShift => Instruction::RightShift,
                    BinaryOperator::NullCoalesce => Instruction::NullCoalesce,
                    BinaryOperator::Pipe => {
                        // Pipe: lhs |> func. Stack: [lhs, rhs_func]. VM pops func, pops arg, calls func(arg).
                        Instruction::Pipe
                    }
                });
            }
            Expression::UnaryOp { op, operand, .. } => {
                match op {
                    UnaryOperator::Increment | UnaryOperator::Decrement => {
                        // ++x / --x: only supported for simple identifier operands.
                        // Emits: load var, push 1, add/subtract, dup (leave on stack), store var.
                        if let Expression::Identifier(name) = operand.as_ref() {
                            let one_idx = self.add_constant(&Literal::Integer(1));
                            instructions.push(Instruction::LoadVar(name.clone()));
                            instructions.push(Instruction::PushConstant(one_idx));
                            if matches!(op, UnaryOperator::Increment) {
                                instructions.push(Instruction::Add);
                            } else {
                                instructions.push(Instruction::Subtract);
                            }
                            instructions.push(Instruction::Dup);
                            instructions.push(Instruction::StoreVar(name.clone()));
                        } else {
                            // Non-identifier operand (e.g. ++arr[0]) — not supported.
                            // Emit a runtime error instead of silently doing nothing.
                            let op_name = if matches!(op, UnaryOperator::Increment) {
                                "++"
                            } else {
                                "--"
                            };
                            let err_idx = self.add_constant(&Literal::String(format!(
                                "{} operator only supports simple variable names (e.g. {}x). Use x = x {} 1 instead.",
                                op_name, op_name, if matches!(op, UnaryOperator::Increment) { "+" } else { "-" }
                            )));
                            instructions.push(Instruction::PushConstant(err_idx));
                            instructions.push(Instruction::Throw);
                        }
                    }
                    _ => {
                        self.compile_expression(operand, instructions);
                        instructions.push(match op {
                            UnaryOperator::Not => Instruction::Not,
                            UnaryOperator::Minus => Instruction::Negate,
                            UnaryOperator::BitNot => Instruction::BitNot,
                            _ => unreachable!(),
                        });
                    }
                }
            }
            Expression::FunctionCall {
                name, arguments, ..
            } => {
                for arg in arguments {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::Call(name.clone(), arguments.len()));
            }
            Expression::Array { elements, .. } => {
                let has_spread = elements
                    .iter()
                    .any(|e| matches!(e, Expression::Spread { .. }));
                if has_spread {
                    // Build with spread: start with empty array, append/extend each element
                    instructions.push(Instruction::BuildArray(0)); // empty array on stack
                    for elem in elements {
                        match elem {
                            Expression::Spread { value, .. } => {
                                self.compile_expression(value, instructions);
                                instructions.push(Instruction::ArrayExtend);
                            }
                            other => {
                                self.compile_expression(other, instructions);
                                instructions.push(Instruction::ArrayAppend);
                            }
                        }
                    }
                } else {
                    for elem in elements {
                        self.compile_expression(elem, instructions);
                    }
                    instructions.push(Instruction::BuildArray(elements.len()));
                }
            }
            Expression::Map { entries, .. } => {
                for (key, value) in entries {
                    self.compile_expression(key, instructions);
                    self.compile_expression(value, instructions);
                }
                instructions.push(Instruction::BuildMap(entries.len()));
            }
            Expression::Index { target, index, .. } => {
                self.compile_expression(target, instructions);
                self.compile_expression(index, instructions);
                instructions.push(Instruction::Index);
            }
            Expression::Member { target, name, .. } => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::GetField(name.clone()));
            }
            Expression::OptionalMember { target, name, .. } => {
                self.compile_expression(target, instructions);
                // Emit a named placeholder; BytecodeVM will raise a clear error at runtime.
                instructions.push(Instruction::OptionalGetField(name.clone()));
            }
            Expression::OptionalCall {
                target, arguments, ..
            } => {
                self.compile_expression(target, instructions);
                for arg in arguments {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::OptionalCall(String::new(), arguments.len()));
            }
            Expression::OptionalIndex { target, index, .. } => {
                self.compile_expression(target, instructions);
                self.compile_expression(index, instructions);
                instructions.push(Instruction::OptionalIndex);
            }
            Expression::InterpolatedString { segments, .. } => {
                // Build string by concatenating segments using Add instructions.
                // Start with an empty string, then add each segment.
                let empty_idx = self.add_constant(&Literal::String(String::new()));
                instructions.push(Instruction::PushConstant(empty_idx));
                for seg in segments {
                    match seg {
                        crate::parser::ast::common::InterpolatedSegment::Text(t) => {
                            let idx = self.add_constant(&Literal::String(t.clone()));
                            instructions.push(Instruction::PushConstant(idx));
                        }
                        crate::parser::ast::common::InterpolatedSegment::Expression(expr) => {
                            self.compile_expression(expr, instructions);
                        }
                    }
                    instructions.push(Instruction::Add); // concatenate
                }
            }
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                self.compile_expression(condition, instructions);
                let false_jump = instructions.len();
                instructions.push(Instruction::Nop); // JumpIfFalse(false_branch)
                self.compile_expression(true_expr, instructions);
                let end_jump = instructions.len();
                instructions.push(Instruction::Nop); // Jump(end)
                let false_start = instructions.len();
                self.compile_expression(false_expr, instructions);
                let end = instructions.len();
                instructions[false_jump] = Instruction::JumpIfFalse(false_start);
                instructions[end_jump] = Instruction::Jump(end);
            }
            Expression::Await { expression, .. } => {
                self.compile_expression(expression, instructions);
                instructions.push(Instruction::Await);
            }
            Expression::Set { elements, .. } => {
                for elem in elements {
                    self.compile_expression(elem, instructions);
                }
                instructions.push(Instruction::BuildSet(elements.len()));
            }
            Expression::Lambda { params, body, .. } => {
                // Compile as anonymous function; no environment capture in bytecode VM.
                // Push a string value holding the internal function name.
                // Call("var_holding_lambda", n) will resolve this via variable lookup.
                let lambda_name = format!("__lambda_{}__", instructions.len());
                let jump_idx = instructions.len();
                instructions.push(Instruction::Nop); // Jump(after_body)
                let body_start = instructions.len();
                self.compile_expression(body, instructions);
                instructions.push(Instruction::ReturnValue);
                let after_body = instructions.len();
                instructions[jump_idx] = Instruction::Jump(after_body);
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                instructions.push(Instruction::RegisterFunction(
                    lambda_name.clone(),
                    param_names,
                    body_start,
                ));
                // Push a FunctionRef (not a plain String) so HOF dispatch can
                // distinguish lambdas from user string values without collision.
                let name_idx = self.add_function_ref(&lambda_name);
                instructions.push(Instruction::PushConstant(name_idx));
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Push object first, then args; CallMethod pops args then object.
                self.compile_expression(object, instructions);
                for arg in arguments {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::CallMethod(method.clone(), arguments.len()));
            }
            Expression::Slice {
                target,
                start,
                end,
                step,
                ..
            } => {
                self.compile_expression(target, instructions);
                match start {
                    Some(e) => self.compile_expression(e, instructions),
                    None => {
                        let i = self.add_constant(&Literal::Null);
                        instructions.push(Instruction::PushConstant(i));
                    }
                }
                match end {
                    Some(e) => self.compile_expression(e, instructions),
                    None => {
                        let i = self.add_constant(&Literal::Null);
                        instructions.push(Instruction::PushConstant(i));
                    }
                }
                match step {
                    Some(e) => self.compile_expression(e, instructions),
                    None => {
                        let i = self.add_constant(&Literal::Null);
                        instructions.push(Instruction::PushConstant(i));
                    }
                }
                instructions.push(Instruction::Slice);
            }
            Expression::StructLiteral { name, fields, .. } => {
                // Build map of field key-value pairs, then SetField each one
                // Approach: build a Map with all fields, then convert to Struct
                // Actually: push field count, then each (key, value), then BuildMap, then TypeCast to struct
                // Simpler: push each key-value pair using string key and value, BuildMap, GetField/Struct
                // For now, emit BuildMap and wrap as a named struct
                let name_idx = self.add_constant(&Literal::String(name.clone()));
                instructions.push(Instruction::PushConstant(name_idx)); // struct type name
                for (field_name, field_expr) in fields {
                    let key_idx = self.add_constant(&Literal::String(field_name.clone()));
                    instructions.push(Instruction::PushConstant(key_idx));
                    self.compile_expression(field_expr, instructions);
                }
                instructions.push(Instruction::BuildStructLiteral(fields.len()));
            }
            Expression::Spread { value, .. } => {
                // Spread outside an array literal — compile inner value (unusual but handle gracefully)
                self.compile_expression(value, instructions);
            }
            Expression::Propagate { value, .. } => {
                // Task 12.6: `?` operator — compile inner expr, then emit Propagate instruction
                self.compile_expression(value, instructions);
                instructions.push(Instruction::Propagate);
            }
        }
    }

    /// Add a `Constant::FunctionRef` to the pool and return its index.
    /// Used when compiling lambdas so the VM can distinguish function
    /// references from plain string values at HOF dispatch time.
    fn add_function_ref(&mut self, name: &str) -> usize {
        let constant = Constant::FunctionRef(name.to_string());
        if let Some(&idx) = self.constant_map.get(&constant) {
            idx
        } else {
            let idx = self.constants.len();
            self.constants.push(constant.clone());
            self.constant_map.insert(constant, idx);
            idx
        }
    }

    fn add_constant(&mut self, lit: &Literal) -> usize {
        let constant = match lit {
            Literal::Integer(i) => Constant::Integer(*i),
            Literal::Float(f) => Constant::Float(*f),
            Literal::String(s) => Constant::String(s.clone()),
            Literal::Boolean(b) => Constant::Boolean(*b),
            Literal::Char(c) => Constant::String(c.to_string()),
            Literal::Null => Constant::Null,
        };

        if let Some(&idx) = self.constant_map.get(&constant) {
            idx
        } else {
            let idx = self.constants.len();
            self.constants.push(constant.clone());
            self.constant_map.insert(constant, idx);
            idx
        }
    }
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}
