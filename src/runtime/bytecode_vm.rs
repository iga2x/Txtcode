use crate::compiler::bytecode::{Bytecode, BytecodeProgram, FunctionInfo};
use crate::runtime::vm::{Value, RuntimeError};
use crate::runtime::gc::GarbageCollector;
use std::collections::HashMap;

/// Stack-based bytecode virtual machine
pub struct BytecodeVM {
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    locals: Vec<HashMap<String, Value>>,
    functions: HashMap<String, FunctionInfo>,
    instructions: Vec<Bytecode>,
    pc: usize, // Program counter
    call_stack: Vec<CallFrame>,
    gc: GarbageCollector,
}

#[derive(Debug, Clone)]
struct CallFrame {
    return_address: usize,
    #[allow(dead_code)] // Reserved for future local variable tracking
    local_vars: HashMap<String, Value>,
    stack_start: usize,
}

impl BytecodeVM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            globals: HashMap::new(),
            locals: Vec::new(),
            functions: HashMap::new(),
            instructions: Vec::new(),
            pc: 0,
            call_stack: Vec::new(),
            gc: GarbageCollector::new(),
        }
    }

    /// Load and execute a bytecode program
    pub fn load(&mut self, program: &BytecodeProgram) {
        self.instructions = program.instructions.clone();
        self.functions = program.functions.clone();
        self.pc = 0;
    }

    /// Execute the loaded bytecode program
    pub fn execute(&mut self) -> Result<Value, RuntimeError> {
        while self.pc < self.instructions.len() {
            let instruction = &self.instructions[self.pc].clone();
            
            match self.execute_instruction(instruction) {
                Ok(ExecutionResult::Continue) => {
                    self.pc += 1;
                }
                Ok(ExecutionResult::Jump(address)) => {
                    self.pc = address;
                }
                Ok(ExecutionResult::Return(value)) => {
                    return Ok(value);
                }
                Ok(ExecutionResult::Halt) => {
                    break;
                }
                Err(e) => return Err(e),
            }

            // Periodic garbage collection
            if self.pc % 100 == 0 {
                self.gc.collect(&mut self.stack, &mut self.globals);
            }
        }

        // Return top of stack or null
        Ok(self.stack.pop().unwrap_or(Value::Null))
    }

    fn execute_instruction(&mut self, instruction: &Bytecode) -> Result<ExecutionResult, RuntimeError> {
        match instruction {
            // Stack operations
            Bytecode::PushInt(n) => {
                self.stack.push(Value::Integer(*n));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::PushFloat(n) => {
                self.stack.push(Value::Float(*n));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::PushString(s) => {
                self.stack.push(Value::String(s.clone()));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::PushBool(b) => {
                self.stack.push(Value::Boolean(*b));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::PushNull => {
                self.stack.push(Value::Null);
                Ok(ExecutionResult::Continue)
            }

            // Variable operations
            Bytecode::LoadVar(name) => {
                let value = self.get_variable(name)?;
                self.stack.push(value);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::StoreVar(name) => {
                let value = self.stack.pop().ok_or_else(|| RuntimeError {
                    message: "Stack underflow".to_string(),
                })?;
                self.set_variable(name, value)?;
                Ok(ExecutionResult::Continue)
            }

            // Arithmetic operations
            Bytecode::Add => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.add_values(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Subtract => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.subtract_values(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Multiply => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.multiply_values(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Divide => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.divide_values(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Modulo => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.modulo_values(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Power => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.power_values(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }

            // Comparison operations
            Bytecode::Equal => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(Value::Boolean(self.values_equal(&left, &right)));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::NotEqual => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(Value::Boolean(!self.values_equal(&left, &right)));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Less => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.compare_values(&left, &right, |a, b| a < b)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Greater => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.compare_values(&left, &right, |a, b| a > b)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::LessEqual => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.compare_values(&left, &right, |a, b| a <= b)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::GreaterEqual => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.compare_values(&left, &right, |a, b| a >= b)?);
                Ok(ExecutionResult::Continue)
            }

            // Logical operations
            Bytecode::And => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(Value::Boolean(self.is_truthy(&left) && self.is_truthy(&right)));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Or => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(Value::Boolean(self.is_truthy(&left) || self.is_truthy(&right)));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Not => {
                let operand = self.pop_value()?;
                self.stack.push(Value::Boolean(!self.is_truthy(&operand)));
                Ok(ExecutionResult::Continue)
            }

            // Bitwise operations
            Bytecode::BitAnd => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.bitwise_and(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::BitOr => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.bitwise_or(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::BitXor => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.bitwise_xor(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::LeftShift => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.left_shift(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::RightShift => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.stack.push(self.right_shift(&left, &right)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::BitNot => {
                let operand = self.pop_value()?;
                self.stack.push(self.bitwise_not(&operand)?);
                Ok(ExecutionResult::Continue)
            }

            // Control flow
            Bytecode::Jump(address) => {
                Ok(ExecutionResult::Jump(*address))
            }
            Bytecode::JumpIfFalse(address) => {
                let condition = self.pop_value()?;
                if !self.is_truthy(&condition) {
                    Ok(ExecutionResult::Jump(*address))
                } else {
                    Ok(ExecutionResult::Continue)
                }
            }
            Bytecode::JumpIfTrue(address) => {
                let condition = self.pop_value()?;
                if self.is_truthy(&condition) {
                    Ok(ExecutionResult::Jump(*address))
                } else {
                    Ok(ExecutionResult::Continue)
                }
            }

            // Function operations
            Bytecode::Call(name, arg_count) => {
                self.call_function(name, *arg_count)?;
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Return => {
                let return_value = self.stack.pop().unwrap_or(Value::Null);
                if let Some(frame) = self.call_stack.pop() {
                    // Restore stack
                    self.stack.truncate(frame.stack_start);
                    self.stack.push(return_value.clone());
                    Ok(ExecutionResult::Jump(frame.return_address))
                } else {
                    Ok(ExecutionResult::Return(return_value))
                }
            }

            // Array/Map operations
            Bytecode::MakeArray(size) => {
                let mut elements = Vec::new();
                for _ in 0..*size {
                    elements.insert(0, self.stack.pop().ok_or_else(|| RuntimeError {
                        message: "Stack underflow".to_string(),
                    })?);
                }
                self.stack.push(Value::Array(elements));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::MakeMap(size) => {
                let mut map = HashMap::new();
                for _ in 0..*size {
                    let value = self.stack.pop().ok_or_else(|| RuntimeError {
                        message: "Stack underflow".to_string(),
                    })?;
                    let key = self.stack.pop().ok_or_else(|| RuntimeError {
                        message: "Stack underflow".to_string(),
                    })?;
                    if let Value::String(key_str) = key {
                        map.insert(key_str, value);
                    } else {
                        return Err(RuntimeError {
                            message: "Map key must be a string".to_string(),
                        });
                    }
                }
                self.stack.push(Value::Map(map));
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Index => {
                let index = self.pop_value()?;
                let target = self.pop_value()?;
                self.stack.push(self.index_value(&target, &index)?);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Member(member) => {
                let target = self.pop_value()?;
                self.stack.push(self.get_member(&target, member)?);
                Ok(ExecutionResult::Continue)
            }

            // Built-in functions
            Bytecode::Print => {
                if let Some(value) = self.stack.last() {
                    println!("{}", value.to_string());
                }
                Ok(ExecutionResult::Continue)
            }

            // Stack utilities
            Bytecode::Pop => {
                self.stack.pop();
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Dup => {
                let top = self.stack.last().ok_or_else(|| RuntimeError {
                    message: "Stack underflow".to_string(),
                })?;
                self.stack.push(top.clone());
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Swap => {
                if self.stack.len() < 2 {
                    return Err(RuntimeError {
                        message: "Stack underflow".to_string(),
                    });
                }
                let len = self.stack.len();
                self.stack.swap(len - 1, len - 2);
                Ok(ExecutionResult::Continue)
            }
            Bytecode::Nop => {
                Ok(ExecutionResult::Continue)
            }
        }
    }

    fn call_function(&mut self, name: &str, arg_count: usize) -> Result<(), RuntimeError> {
        // Built-in functions
        match name {
            "print" => {
                if arg_count > 0 {
                    let value = self.stack.pop().ok_or_else(|| RuntimeError {
                        message: "Stack underflow".to_string(),
                    })?;
                    println!("{}", value.to_string());
                }
                self.stack.push(Value::Null);
                return Ok(());
            }
            _ => {}
        }

        // Standard library functions
        let args: Vec<Value> = {
            let mut args = Vec::new();
            for _ in 0..arg_count {
                args.insert(0, self.stack.pop().ok_or_else(|| RuntimeError {
                    message: "Stack underflow".to_string(),
                })?);
            }
            args
        };

        // Try standard library
        if let Ok(result) = crate::stdlib::StdLib::call_function(name, &args) {
            self.stack.push(result);
            return Ok(());
        }

        // User-defined functions
        if let Some(func_info) = self.functions.get(name) {
            let stack_start = self.stack.len() - arg_count;
            
            // Create call frame
            let frame = CallFrame {
                return_address: self.pc + 1,
                local_vars: HashMap::new(),
                stack_start,
            };
            self.call_stack.push(frame);
            
            // Set up local variables from arguments
            // (Simplified - in full implementation, would map to parameter names)
            
            // Jump to function
            self.pc = func_info.address;
        } else {
            return Err(RuntimeError {
                message: format!("Function '{}' not found", name),
            });
        }

        Ok(())
    }

    fn get_variable(&self, name: &str) -> Result<Value, RuntimeError> {
        // Check local scope first
        if let Some(locals) = self.locals.last() {
            if let Some(value) = locals.get(name) {
                return Ok(value.clone());
            }
        }
        
        // Check globals
        if let Some(value) = self.globals.get(name) {
            return Ok(value.clone());
        }

        Err(RuntimeError {
            message: format!("Undefined variable: {}", name),
        })
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        // Set in local scope if available
        if let Some(locals) = self.locals.last_mut() {
            locals.insert(name.to_string(), value);
            return Ok(());
        }
        
        // Otherwise set in globals
        self.globals.insert(name.to_string(), value);
        Ok(())
    }

    fn pop_value(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(|| RuntimeError {
            message: "Stack underflow".to_string(),
        })
    }

    // Value operations (reuse from tree-walk VM)
    fn add_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b.to_string()))),
            (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a.to_string(), b))),
            _ => Err(RuntimeError {
                message: format!("Cannot add {} and {}", left.type_name(), right.type_name()),
            }),
        }
    }

    fn subtract_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(RuntimeError {
                message: format!("Cannot subtract {} from {}", right.type_name(), left.type_name()),
            }),
        }
    }

    fn multiply_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(RuntimeError {
                message: format!("Cannot multiply {} and {}", left.type_name(), right.type_name()),
            }),
        }
    }

    fn divide_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError { message: "Division by zero".to_string() })
                } else {
                    Ok(Value::Float(*a as f64 / *b as f64))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError { message: "Division by zero".to_string() })
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            _ => Err(RuntimeError {
                message: format!("Cannot divide {} by {}", left.type_name(), right.type_name()),
            }),
        }
    }

    fn modulo_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError { message: "Modulo by zero".to_string() })
                } else {
                    Ok(Value::Integer(a % b))
                }
            }
            _ => Err(RuntimeError {
                message: "Modulo requires integers".to_string(),
            }),
        }
    }

    fn power_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                Ok(Value::Float((*a as f64).powf(*b as f64)))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
            _ => Err(RuntimeError {
                message: "Power requires numbers".to_string(),
            }),
        }
    }

    fn compare_values<F>(&self, left: &Value, right: &Value, cmp: F) -> Result<Value, RuntimeError>
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(cmp(*a as f64, *b as f64))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(cmp(*a, *b))),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean(cmp(*a as f64, *b))),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(cmp(*a, *b as f64))),
            _ => Err(RuntimeError {
                message: format!("Cannot compare {} and {}", left.type_name(), right.type_name()),
            }),
        }
    }

    fn bitwise_and(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a & b)),
            _ => Err(RuntimeError {
                message: "Bitwise operations require integers".to_string(),
            }),
        }
    }

    fn bitwise_or(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a | b)),
            _ => Err(RuntimeError {
                message: "Bitwise operations require integers".to_string(),
            }),
        }
    }

    fn bitwise_xor(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a ^ b)),
            _ => Err(RuntimeError {
                message: "Bitwise operations require integers".to_string(),
            }),
        }
    }

    fn left_shift(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a << b)),
            _ => Err(RuntimeError {
                message: "Bitwise shift requires integers".to_string(),
            }),
        }
    }

    fn right_shift(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a >> b)),
            _ => Err(RuntimeError {
                message: "Bitwise shift requires integers".to_string(),
            }),
        }
    }

    fn bitwise_not(&self, operand: &Value) -> Result<Value, RuntimeError> {
        match operand {
            Value::Integer(n) => Ok(Value::Integer(!n)),
            _ => Err(RuntimeError {
                message: "Bitwise not requires integer".to_string(),
            }),
        }
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Null => false,
            Value::Integer(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Map(map) => !map.is_empty(),
            Value::Function { .. } => true,
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn index_value(&self, target: &Value, index: &Value) -> Result<Value, RuntimeError> {
        match (target, index) {
            (Value::Array(arr), Value::Integer(i)) => {
                let idx = *i as usize;
                if idx < arr.len() {
                    Ok(arr[idx].clone())
                } else {
                    Err(RuntimeError {
                        message: format!("Index {} out of bounds", idx),
                    })
                }
            }
            (Value::Map(map), Value::String(key)) => {
                map.get(key).cloned().ok_or_else(|| RuntimeError {
                    message: format!("Key '{}' not found in map", key),
                })
            }
            _ => Err(RuntimeError {
                message: format!("Cannot index {} with {}", target.type_name(), index.type_name()),
            }),
        }
    }

    fn get_member(&self, target: &Value, member: &str) -> Result<Value, RuntimeError> {
        match target {
            Value::Map(map) => {
                map.get(member).cloned().ok_or_else(|| RuntimeError {
                    message: format!("Member '{}' not found", member),
                })
            }
            _ => Err(RuntimeError {
                message: format!("Cannot access member '{}' on {}", member, target.type_name()),
            }),
        }
    }
}

#[derive(Debug)]
enum ExecutionResult {
    Continue,
    Jump(usize),
    Return(Value),
    #[allow(dead_code)] // Reserved for future graceful shutdown
    Halt,
}

impl Default for BytecodeVM {
    fn default() -> Self {
        Self::new()
    }
}

