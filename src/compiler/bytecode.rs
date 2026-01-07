use crate::parser::ast::*;
use crate::runtime::vm::Value;
use std::collections::HashMap;

/// Bytecode instruction set
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Bytecode {
    // Stack operations
    PushInt(i64),
    PushFloat(f64),
    PushString(String),
    PushBool(bool),
    PushNull,
    
    // Variable operations
    LoadVar(String),
    StoreVar(String),
    
    // Arithmetic operations
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    
    // Comparison operations
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    
    // Logical operations
    And,
    Or,
    Not,
    
    // Bitwise operations
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    BitNot,
    
    // Control flow
    Jump(usize),              // Unconditional jump
    JumpIfFalse(usize),       // Jump if top of stack is false
    JumpIfTrue(usize),        // Jump if top of stack is true
    
    // Function operations
    Call(String, usize),      // Call function with n arguments
    Return,
    
    // Array/Map operations
    MakeArray(usize),         // Create array with n elements
    MakeMap(usize),           // Create map with n key-value pairs
    Index,                    // Index operation (array[index] or map[key])
    Member(String),           // Member access (object.member)
    
    // Built-in functions
    Print,
    
    // Special
    Pop,                      // Pop top of stack
    Dup,                      // Duplicate top of stack
    Swap,                     // Swap top two stack elements
    Nop,                      // No operation
}

/// Compiled bytecode program
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BytecodeProgram {
    pub instructions: Vec<Bytecode>,
    pub constants: Vec<Value>,
    pub functions: HashMap<String, FunctionInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionInfo {
    pub address: usize,
    pub param_count: usize,
    pub local_count: usize,
}

/// Bytecode compiler
pub struct BytecodeCompiler {
    instructions: Vec<Bytecode>,
    constants: Vec<Value>,
    functions: HashMap<String, FunctionInfo>,
    label_counter: usize,
    labels: HashMap<String, usize>,
    patch_list: Vec<(usize, String)>, // (instruction_index, label_name)
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            label_counter: 0,
            labels: HashMap::new(),
            patch_list: Vec::new(),
        }
    }

    /// Compile AST program to bytecode
    pub fn compile(&mut self, program: &Program) -> BytecodeProgram {
        self.instructions.clear();
        self.constants.clear();
        self.functions.clear();
        self.label_counter = 0;
        self.labels.clear();
        self.patch_list.clear();

        // First pass: collect function definitions
        for statement in &program.statements {
            if let Statement::FunctionDef { name, params, .. } = statement {
                let label = format!("func_{}", name);
                self.labels.insert(label.clone(), self.instructions.len());
                self.functions.insert(name.clone(), FunctionInfo {
                    address: self.instructions.len(),
                    param_count: params.len(),
                    local_count: 0,
                });
            }
        }

        // Second pass: compile statements
        for statement in &program.statements {
            self.compile_statement(statement);
        }

        // Patch jump addresses
        self.patch_jumps();

        BytecodeProgram {
            instructions: self.instructions.clone(),
            constants: self.constants.clone(),
            functions: self.functions.clone(),
        }
    }

    fn compile_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Expression(expr) => {
                self.compile_expression(expr);
                self.emit(Bytecode::Pop); // Discard expression result
            }
            Statement::Assignment { name, value, .. } => {
                self.compile_expression(value);
                self.emit(Bytecode::StoreVar(name.clone()));
            }
            Statement::FunctionDef { name: _name, params: _params, body: _body, .. } => {
                // Function definition - skip for now, already collected in first pass
                // In a full implementation, we'd compile the function body here
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expression(expr);
                } else {
                    self.emit(Bytecode::PushNull);
                }
                self.emit(Bytecode::Return);
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
                self.compile_expression(condition);
                
                let else_label = self.new_label();
                let end_label = self.new_label();
                
                // Jump to else if condition is false
                self.emit_jump(Bytecode::JumpIfFalse(0), else_label.clone());
                
                // Compile then branch
                for stmt in then_branch {
                    self.compile_statement(stmt);
                }
                
                // Jump to end after then branch
                self.emit_jump(Bytecode::Jump(0), end_label.clone());
                
                // Emit else label
                self.emit_label(else_label.clone());
                
                // Compile elseif branches
                for (elseif_cond, elseif_body) in else_if_branches {
                    self.compile_expression(elseif_cond);
                    let next_else_label = self.new_label();
                    self.emit_jump(Bytecode::JumpIfFalse(0), next_else_label.clone());
                    
                    for stmt in elseif_body {
                        self.compile_statement(stmt);
                    }
                    
                    self.emit_jump(Bytecode::Jump(0), end_label.clone());
                    self.emit_label(next_else_label);
                }
                
                // Compile else branch
                if let Some(else_body) = else_branch {
                    for stmt in else_body {
                        self.compile_statement(stmt);
                    }
                }
                
                // Emit end label
                self.emit_label(end_label);
            }
            Statement::While { condition, body, .. } => {
                let loop_label = self.new_label();
                let end_label = self.new_label();
                
                // Emit loop start
                self.emit_label(loop_label.clone());
                
                // Compile condition
                self.compile_expression(condition);
                self.emit_jump(Bytecode::JumpIfFalse(0), end_label.clone());
                
                // Compile body
                for stmt in body {
                    self.compile_statement(stmt);
                }
                
                // Jump back to loop start
                self.emit_jump(Bytecode::Jump(0), loop_label.clone());
                
                // Emit end label
                self.emit_label(end_label);
            }
            Statement::For { variable: _variable, iterable, body, .. } => {
                // Compile iterable
                self.compile_expression(iterable);
                
                // For loop implementation would require iterator support
                // Simplified version for now
                let loop_label = self.new_label();
                let end_label = self.new_label();
                
                self.emit_label(loop_label.clone());
                // Iterator logic would go here
                self.emit_jump(Bytecode::Jump(0), end_label.clone());
                
                for stmt in body {
                    self.compile_statement(stmt);
                }
                
                self.emit_jump(Bytecode::Jump(0), loop_label.clone());
                self.emit_label(end_label);
            }
            Statement::Repeat { count, body, .. } => {
                // Compile count
                self.compile_expression(count);
                
                let loop_label = self.new_label();
                let end_label = self.new_label();
                
                // Store counter
                let counter_var = format!("_repeat_counter_{}", self.label_counter);
                self.emit(Bytecode::StoreVar(counter_var.clone()));
                
                // Loop start
                self.emit_label(loop_label.clone());
                
                // Load counter and check if > 0
                self.emit(Bytecode::LoadVar(counter_var.clone()));
                self.emit(Bytecode::PushInt(0));
                self.emit(Bytecode::LessEqual);
                self.emit_jump(Bytecode::JumpIfTrue(0), end_label.clone());
                
                // Decrement counter
                self.emit(Bytecode::LoadVar(counter_var.clone()));
                self.emit(Bytecode::PushInt(1));
                self.emit(Bytecode::Subtract);
                self.emit(Bytecode::StoreVar(counter_var.clone()));
                
                // Compile body
                for stmt in body {
                    self.compile_statement(stmt);
                }
                
                // Jump back to loop
                self.emit_jump(Bytecode::Jump(0), loop_label.clone());
                
                // End label
                self.emit_label(end_label);
            }
            Statement::Match { value, cases, default, .. } => {
                self.compile_expression(value);
                
                let end_label = self.new_label();
                let mut case_labels = Vec::new();
                
                // Compile each case
                for case in cases {
                    let case_label = self.new_label();
                    case_labels.push(case_label.clone());
                    
                    // Match pattern (simplified - would need pattern matching logic)
                    self.emit(Bytecode::Dup); // Keep value on stack
                    // Pattern matching would go here
                    
                    // Check guard if present
                    if let Some(guard) = &case.guard {
                        self.compile_expression(guard);
                        let next_case = self.new_label();
                        self.emit_jump(Bytecode::JumpIfFalse(0), next_case.clone());
                        
                        // Compile case body
                        for stmt in &case.body {
                            self.compile_statement(stmt);
                        }
                        
                        self.emit_jump(Bytecode::Jump(0), end_label.clone());
                        self.emit_label(next_case);
                    } else {
                        // Compile case body
                        for stmt in &case.body {
                            self.compile_statement(stmt);
                        }
                        self.emit_jump(Bytecode::Jump(0), end_label.clone());
                    }
                    
                    self.emit_label(case_label);
                }
                
                // Default case
                if let Some(default_body) = default {
                    for stmt in default_body {
                        self.compile_statement(stmt);
                    }
                }
                
                self.emit_label(end_label);
            }
            Statement::Break { .. } => {
                // Break would need to jump to end of loop
                // Simplified for now
                self.emit(Bytecode::Nop);
            }
            Statement::Continue { .. } => {
                // Continue would need to jump to start of loop
                // Simplified for now
                self.emit(Bytecode::Nop);
            }
            Statement::Try { body, catch, .. } => {
                // Exception handling would require exception mechanism
                // Simplified: just compile body
                for stmt in body {
                    self.compile_statement(stmt);
                }
                if let Some((_, catch_body)) = catch {
                    // Catch block would be compiled here
                    for stmt in catch_body {
                        self.compile_statement(stmt);
                    }
                }
            }
            Statement::Import { .. } => {
                // Import handling
                self.emit(Bytecode::Nop);
            }
        }
    }

    fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(lit) => {
                match lit {
                    Literal::Integer(n) => self.emit(Bytecode::PushInt(*n)),
                    Literal::Float(n) => self.emit(Bytecode::PushFloat(*n)),
                    Literal::String(s) => self.emit(Bytecode::PushString(s.clone())),
                    Literal::Boolean(b) => self.emit(Bytecode::PushBool(*b)),
                    Literal::Null => self.emit(Bytecode::PushNull),
                }
            }
            Expression::Identifier(name) => {
                self.emit(Bytecode::LoadVar(name.clone()));
            }
            Expression::BinaryOp { left, op, right, .. } => {
                self.compile_expression(left);
                self.compile_expression(right);
                
                match op {
                    BinaryOperator::Add => self.emit(Bytecode::Add),
                    BinaryOperator::Subtract => self.emit(Bytecode::Subtract),
                    BinaryOperator::Multiply => self.emit(Bytecode::Multiply),
                    BinaryOperator::Divide => self.emit(Bytecode::Divide),
                    BinaryOperator::Modulo => self.emit(Bytecode::Modulo),
                    BinaryOperator::Power => self.emit(Bytecode::Power),
                    BinaryOperator::Equal => self.emit(Bytecode::Equal),
                    BinaryOperator::NotEqual => self.emit(Bytecode::NotEqual),
                    BinaryOperator::Less => self.emit(Bytecode::Less),
                    BinaryOperator::Greater => self.emit(Bytecode::Greater),
                    BinaryOperator::LessEqual => self.emit(Bytecode::LessEqual),
                    BinaryOperator::GreaterEqual => self.emit(Bytecode::GreaterEqual),
                    BinaryOperator::And => self.emit(Bytecode::And),
                    BinaryOperator::Or => self.emit(Bytecode::Or),
                    BinaryOperator::BitAnd => self.emit(Bytecode::BitAnd),
                    BinaryOperator::BitOr => self.emit(Bytecode::BitOr),
                    BinaryOperator::BitXor => self.emit(Bytecode::BitXor),
                    BinaryOperator::LeftShift => self.emit(Bytecode::LeftShift),
                    BinaryOperator::RightShift => self.emit(Bytecode::RightShift),
                    BinaryOperator::Arrow => {
                        // Arrow operator for function calls
                        // Would need special handling
                        self.emit(Bytecode::Nop);
                    }
                }
            }
            Expression::UnaryOp { op, operand, .. } => {
                self.compile_expression(operand);
                
                match op {
                    UnaryOperator::Not => self.emit(Bytecode::Not),
                    UnaryOperator::Minus => {
                        self.emit(Bytecode::PushInt(0));
                        self.emit(Bytecode::Swap);
                        self.emit(Bytecode::Subtract);
                    }
                    UnaryOperator::BitNot => self.emit(Bytecode::BitNot),
                }
            }
            Expression::FunctionCall { name, arguments, .. } => {
                // Compile arguments in reverse order
                for arg in arguments.iter().rev() {
                    self.compile_expression(arg);
                }
                
                // Call function
                self.emit(Bytecode::Call(name.clone(), arguments.len()));
            }
            Expression::Array { elements, .. } => {
                // Compile elements
                for elem in elements {
                    self.compile_expression(elem);
                }
                self.emit(Bytecode::MakeArray(elements.len()));
            }
            Expression::Map { entries, .. } => {
                // Compile key-value pairs
                for (key, value) in entries {
                    self.emit(Bytecode::PushString(key.clone()));
                    self.compile_expression(value);
                }
                self.emit(Bytecode::MakeMap(entries.len()));
            }
            Expression::Index { target, index, .. } => {
                self.compile_expression(target);
                self.compile_expression(index);
                self.emit(Bytecode::Index);
            }
            Expression::Member { target, member, .. } => {
                self.compile_expression(target);
                self.emit(Bytecode::Member(member.clone()));
            }
            Expression::Lambda { params: _params, body: _body, .. } => {
                // Lambda compilation would create a closure
                // Simplified for now
                self.emit(Bytecode::PushNull);
            }
        }
    }

    fn emit(&mut self, instruction: Bytecode) {
        self.instructions.push(instruction);
    }

    fn new_label(&mut self) -> String {
        let label = format!("L{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    fn emit_label(&mut self, label: String) {
        self.labels.insert(label.clone(), self.instructions.len());
    }

    fn emit_jump(&mut self, instruction: Bytecode, label: String) {
        let index = self.instructions.len();
        self.instructions.push(instruction);
        self.patch_list.push((index, label));
    }

    fn patch_jumps(&mut self) {
        for (index, label) in &self.patch_list {
            if let Some(target) = self.labels.get(label) {
                match &mut self.instructions[*index] {
                    Bytecode::Jump(addr) => *addr = *target,
                    Bytecode::JumpIfFalse(addr) => *addr = *target,
                    Bytecode::JumpIfTrue(addr) => *addr = *target,
                    _ => {}
                }
            }
        }
    }
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}
