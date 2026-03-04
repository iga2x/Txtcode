use crate::parser::ast::*;
use serde::{Serialize, Deserialize};

/// Bytecode representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bytecode {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,
}

/// Constant pool for bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
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
            _ => false,
        }
    }
}

/// Bytecode instructions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Instruction {
    // Stack operations
    PushConstant(usize),  // Push constant from constant pool
    Pop,                   // Pop top of stack
    Dup,                   // Duplicate top of stack
    
    // Variable operations
    LoadVar(String),       // Load variable onto stack
    StoreVar(String),      // Store top of stack to variable
    LoadGlobal(String),    // Load global variable
    
    // Arithmetic operations
    Add,                   // Pop two values, push sum
    Subtract,              // Pop two values, push difference
    Multiply,              // Pop two values, push product
    Divide,                // Pop two values, push quotient
    Modulo,                // Pop two values, push remainder
    Power,                 // Pop two values, push power
    Negate,                // Pop value, push negated
    
    // Comparison operations
    Equal,                 // Pop two values, push equality
    NotEqual,              // Pop two values, push inequality
    Less,                  // Pop two values, push less than
    Greater,               // Pop two values, push greater than
    LessEqual,             // Pop two values, push less or equal
    GreaterEqual,          // Pop two values, push greater or equal
    
    // Logical operations
    And,                   // Pop two values, push AND
    Or,                    // Pop two values, push OR
    Not,                   // Pop value, push NOT
    
    // Bitwise operations
    BitAnd,                // Pop two values, push bitwise AND
    BitOr,                 // Pop two values, push bitwise OR
    BitXor,                // Pop two values, push bitwise XOR
    LeftShift,             // Pop two values, push left shift
    RightShift,            // Pop two values, push right shift
    BitNot,                // Pop value, push bitwise NOT
    
    // Control flow
    Jump(usize),           // Unconditional jump to instruction index
    JumpIfFalse(usize),    // Pop value, jump if false
    JumpIfTrue(usize),     // Pop value, jump if true
    
    // Function operations
    Call(String, usize),   // Call function with name and arg count
    Return,                // Return from function
    ReturnValue,           // Pop value and return it
    
    // Array/Map operations
    BuildArray(usize),     // Pop N values, build array
    BuildMap(usize),       // Pop 2N values (key-value pairs), build map
    Index,                 // Pop index/key and object, push indexed value
    SetIndex,              // Pop value, index/key, and object, set indexed value
    
    // Object operations
    GetField(String),      // Pop object, push field value
    SetField(String),      // Pop value and object, set field
    
    // Type operations
    TypeOf,                // Pop value, push type string
    
    // Control flow helpers
    Label(usize),          // Label for jump targets
    
    // No operation
    Nop,
}

/// Bytecode compiler
pub struct BytecodeCompiler {
    constants: Vec<Constant>,
    constant_map: std::collections::HashMap<Constant, usize>,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            constant_map: std::collections::HashMap::new(),
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
        }
    }

    fn compile_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>) {
        match stmt {
            Statement::Assignment { pattern, value, .. } => {
                self.compile_expression(value, instructions);
                // For now, handle simple identifier patterns
                match pattern {
                    Pattern::Identifier(name) => {
                        instructions.push(Instruction::StoreVar(name.clone()));
                    }
                    Pattern::Array(_) | Pattern::Struct { .. } | Pattern::Constructor { .. } | Pattern::Ignore => {
                        // Complex patterns are handled at runtime
                        // For now, just pop the value (it will be handled by runtime)
                        instructions.push(Instruction::Pop);
                    }
                }
            }
            Statement::FunctionDef { name: _name, params: _params, body, .. } => {
                // Function definitions are stored separately
                // For now, just compile the body
                for body_stmt in body {
                    self.compile_statement(body_stmt, instructions);
                }
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
                instructions.push(Instruction::Pop); // Discard result
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.compile_expression(condition, instructions);
                let jump_if_false_idx = instructions.len();
                instructions.push(Instruction::Nop); // Placeholder for jump
                
                // Compile then branch
                for stmt in then_branch {
                    self.compile_statement(stmt, instructions);
                }
                
                let jump_to_end_idx = instructions.len();
                instructions.push(Instruction::Nop); // Placeholder for jump
                
                // Update jump_if_false to jump to else or end
                let else_start = instructions.len();
                if let Some(else_branch) = else_branch {
                    for stmt in else_branch {
                        self.compile_statement(stmt, instructions);
                    }
                }
                let end_idx = instructions.len();
                
                // Update jumps
                instructions[jump_if_false_idx] = Instruction::JumpIfFalse(else_start);
                instructions[jump_to_end_idx] = Instruction::Jump(end_idx);
            }
            Statement::While { condition, body, .. } => {
                let loop_start = instructions.len();
                self.compile_expression(condition, instructions);
                let jump_if_false_idx = instructions.len();
                instructions.push(Instruction::Nop); // Placeholder
                
                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }
                instructions.push(Instruction::Jump(loop_start));
                
                let end_idx = instructions.len();
                instructions[jump_if_false_idx] = Instruction::JumpIfFalse(end_idx);
            }
            Statement::DoWhile { body, condition, .. } => {
                let loop_start = instructions.len();
                
                // Execute body first
                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }
                
                // Check condition at end
                self.compile_expression(condition, instructions);
                let jump_if_true_idx = instructions.len();
                instructions.push(Instruction::Nop); // Placeholder
                
                let _end_idx = instructions.len();
                // Jump back to start if condition is true
                instructions[jump_if_true_idx] = Instruction::JumpIfTrue(loop_start);
            }
            Statement::For { variable: _variable, iterable, body, .. } => {
                self.compile_expression(iterable, instructions);
                // For loop implementation would need iterator support
                // Simplified version for now
                for stmt in body {
                    self.compile_statement(stmt, instructions);
                }
            }
            Statement::Break { .. } => {
                // Break needs loop context - simplified for now
                instructions.push(Instruction::Nop);
            }
            Statement::Continue { .. } => {
                // Continue needs loop context - simplified for now
                instructions.push(Instruction::Nop);
            }
            _ => {
                // Other statements - placeholder
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
            Expression::BinaryOp { left, op, right, .. } => {
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
                    BinaryOperator::NullCoalesce => Instruction::Nop, // TODO: Implement null coalesce in bytecode
                });
            }
            Expression::UnaryOp { op, operand, .. } => {
                self.compile_expression(operand, instructions);
                instructions.push(match op {
                    UnaryOperator::Not => Instruction::Not,
                    UnaryOperator::Minus => Instruction::Negate,
                    UnaryOperator::BitNot => Instruction::BitNot,
                    UnaryOperator::Increment => Instruction::Nop, // TODO: Implement increment in bytecode
                    UnaryOperator::Decrement => Instruction::Nop, // TODO: Implement decrement in bytecode
                });
            }
            Expression::FunctionCall { name, arguments, .. } => {
                for arg in arguments {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::Call(name.clone(), arguments.len()));
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    self.compile_expression(elem, instructions);
                }
                instructions.push(Instruction::BuildArray(elements.len()));
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
            Expression::OptionalMember { target, name: _name, .. } => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::Nop); // TODO: Implement optional member in bytecode
            }
            Expression::OptionalCall { target, arguments, .. } => {
                self.compile_expression(target, instructions);
                for arg in arguments {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::Nop); // TODO: Implement optional call in bytecode
            }
            Expression::OptionalIndex { target, index, .. } => {
                self.compile_expression(target, instructions);
                self.compile_expression(index, instructions);
                instructions.push(Instruction::Nop); // TODO: Implement optional index in bytecode
            }
            _ => {
                // Other expressions - placeholder
                instructions.push(Instruction::Nop);
            }
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
