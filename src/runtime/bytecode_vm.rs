use crate::compiler::bytecode::{Bytecode, Instruction, Constant};
use crate::runtime::core::{Value, ScopeManager};
use crate::runtime::errors::RuntimeError;
use crate::stdlib::{StdLib, FunctionExecutor};
use std::collections::HashMap;

/// Bytecode Virtual Machine
/// Executes compiled bytecode instructions
pub struct BytecodeVM {
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
    scope_manager: ScopeManager,
    #[allow(dead_code)]
    functions: HashMap<String, (Vec<String>, Vec<Instruction>)>, // name -> (params, body)
    ip: usize, // Instruction pointer
    call_stack: Vec<(usize, HashMap<String, Value>)>, // (return_ip, local_vars)
}

impl BytecodeVM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
            scope_manager: ScopeManager::new(),
            functions: HashMap::new(),
            ip: 0,
            call_stack: Vec::new(),
        }
    }

    /// Execute bytecode
    pub fn execute(&mut self, bytecode: &Bytecode) -> Result<Value, RuntimeError> {
        self.ip = 0;
        
        while self.ip < bytecode.instructions.len() {
            let instruction = &bytecode.instructions[self.ip];
            self.execute_instruction(instruction, &bytecode.constants)?;
            self.ip += 1;
        }
        
        Ok(self.stack.pop().unwrap_or(Value::Null))
    }

    fn execute_instruction(&mut self, inst: &Instruction, constants: &[Constant]) -> Result<(), RuntimeError> {
        match inst {
            Instruction::PushConstant(idx) => {
                let constant = &constants[*idx];
                let value = self.constant_to_value(constant);
                self.stack.push(value);
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Dup => {
                if let Some(val) = self.stack.last() {
                    self.stack.push(val.clone());
                }
            }
            Instruction::LoadVar(name) => {
                let value = self.variables.get(name)
                    .cloned()
                    .or_else(|| self.scope_manager.get_variable(name))
                    .ok_or_else(|| RuntimeError::new(format!("Undefined variable: {}", name)))?;
                self.stack.push(value);
            }
            Instruction::StoreVar(name) => {
                let value = self.stack.pop()
                    .ok_or_else(|| RuntimeError::new("Stack underflow".to_string()))?;
                self.variables.insert(name.clone(), value);
            }
            Instruction::LoadGlobal(name) => {
                let value = self.scope_manager.get_variable(name)
                    .ok_or_else(|| RuntimeError::new(format!("Undefined global: {}", name)))?
                    .clone();
                self.stack.push(value);
            }
            Instruction::Add => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(self.binary_op(a, b, |a, b| a + b)?);
            }
            Instruction::Subtract => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(self.binary_op(a, b, |a, b| a - b)?);
            }
            Instruction::Multiply => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(self.binary_op(a, b, |a, b| a * b)?);
            }
            Instruction::Divide => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    if *b_val == 0 {
                        return Err(RuntimeError::new("Division by zero".to_string()));
                    }
                    self.stack.push(Value::Integer(a_val / b_val));
                } else if let (Value::Float(a_val), Value::Float(b_val)) = (a, b) {
                    if b_val == 0.0 {
                        return Err(RuntimeError::new("Division by zero".to_string()));
                    }
                    self.stack.push(Value::Float(a_val / b_val));
                } else {
                    return Err(RuntimeError::new("Type mismatch in division".to_string()));
                }
            }
            Instruction::Modulo => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    if *b_val == 0 {
                        return Err(RuntimeError::new("Modulo by zero".to_string()));
                    }
                    self.stack.push(Value::Integer(a_val % b_val));
                } else {
                    return Err(RuntimeError::new("Modulo requires integers".to_string()));
                }
            }
            Instruction::Power => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                match (&a, &b) {
                    (Value::Integer(a_val), Value::Integer(b_val)) => {
                        self.stack.push(Value::Integer(a_val.pow(*b_val as u32)));
                    }
                    (Value::Float(a_val), Value::Float(b_val)) => {
                        self.stack.push(Value::Float(a_val.powf(*b_val)));
                    }
                    _ => return Err(RuntimeError::new("Power requires numbers".to_string())),
                }
            }
            Instruction::Negate => {
                let val = self.pop_value()?;
                match val {
                    Value::Integer(i) => self.stack.push(Value::Integer(-i)),
                    Value::Float(f) => self.stack.push(Value::Float(-f)),
                    _ => return Err(RuntimeError::new("Negate requires number".to_string())),
                }
            }
            Instruction::Equal => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(a == b));
            }
            Instruction::NotEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(a != b));
            }
            Instruction::Less => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? < 0));
            }
            Instruction::Greater => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? > 0));
            }
            Instruction::LessEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? <= 0));
            }
            Instruction::GreaterEqual => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                self.stack.push(Value::Boolean(self.compare(&a, &b)? >= 0));
            }
            Instruction::And => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let a_bool = self.to_bool(&a)?;
                let b_bool = self.to_bool(&b)?;
                self.stack.push(Value::Boolean(a_bool && b_bool));
            }
            Instruction::Or => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                let a_bool = self.to_bool(&a)?;
                let b_bool = self.to_bool(&b)?;
                self.stack.push(Value::Boolean(a_bool || b_bool));
            }
            Instruction::Not => {
                let val = self.pop_value()?;
                let bool_val = self.to_bool(&val)?;
                self.stack.push(Value::Boolean(!bool_val));
            }
            Instruction::BitAnd => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val & b_val));
                } else {
                    return Err(RuntimeError::new("Bitwise AND requires integers".to_string()));
                }
            }
            Instruction::BitOr => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val | b_val));
                } else {
                    return Err(RuntimeError::new("Bitwise OR requires integers".to_string()));
                }
            }
            Instruction::BitXor => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val ^ b_val));
                } else {
                    return Err(RuntimeError::new("Bitwise XOR requires integers".to_string()));
                }
            }
            Instruction::LeftShift => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val << b_val));
                } else {
                    return Err(RuntimeError::new("Left shift requires integers".to_string()));
                }
            }
            Instruction::RightShift => {
                let b = self.pop_value()?;
                let a = self.pop_value()?;
                if let (Value::Integer(a_val), Value::Integer(b_val)) = (&a, &b) {
                    self.stack.push(Value::Integer(a_val >> b_val));
                } else {
                    return Err(RuntimeError::new("Right shift requires integers".to_string()));
                }
            }
            Instruction::BitNot => {
                let val = self.pop_value()?;
                if let Value::Integer(i) = val {
                    self.stack.push(Value::Integer(!i));
                } else {
                    return Err(RuntimeError::new("Bitwise NOT requires integer".to_string()));
                }
            }
            Instruction::Jump(target) => {
                self.ip = *target;
            }
            Instruction::JumpIfFalse(target) => {
                let val = self.pop_value()?;
                if !self.to_bool(&val)? {
                    self.ip = *target;
                }
            }
            Instruction::JumpIfTrue(target) => {
                let val = self.pop_value()?;
                if self.to_bool(&val)? {
                    self.ip = *target;
                }
            }
            Instruction::Call(name, arg_count) => {
                // Try stdlib first
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.insert(0, self.pop_value()?);
                }
                
                // Use a dummy executor type - bytecode VM doesn't need executor
                struct DummyExecutor;
                impl FunctionExecutor for DummyExecutor {
                    fn call_function_value(&mut self, _func: &Value, _args: &[Value]) -> Result<Value, RuntimeError> {
                        Err(RuntimeError::new("Not implemented in bytecode VM".to_string()))
                    }
                }
                
                match StdLib::call_function::<DummyExecutor>(name, &args, true, None) {
                    Ok(result) => self.stack.push(result),
                    Err(_) => {
                        // Not a stdlib function - would need function lookup
                        return Err(RuntimeError::new(format!("Function not found: {}", name)));
                    }
                }
            }
            Instruction::Return => {
                // Return from function
                if let Some((return_ip, _)) = self.call_stack.pop() {
                    self.ip = return_ip;
                } else {
                    return Err(RuntimeError::new("Return outside function".to_string()));
                }
            }
            Instruction::ReturnValue => {
                let value = self.pop_value()?;
                if let Some((return_ip, _)) = self.call_stack.pop() {
                    self.stack.push(value);
                    self.ip = return_ip;
                } else {
                    return Err(RuntimeError::new("Return outside function".to_string()));
                }
            }
            Instruction::BuildArray(count) => {
                let mut arr = Vec::new();
                for _ in 0..*count {
                    arr.insert(0, self.pop_value()?);
                }
                self.stack.push(Value::Array(arr));
            }
            Instruction::BuildMap(count) => {
                let mut map = HashMap::new();
                for _ in 0..*count {
                    let value = self.pop_value()?;
                    let key_val = self.pop_value()?;
                    if let Value::String(key) = key_val {
                        map.insert(key, value);
                    } else {
                        return Err(RuntimeError::new("Map keys must be strings".to_string()));
                    }
                }
                self.stack.push(Value::Map(map));
            }
            Instruction::Index => {
                let index = self.pop_value()?;
                let target = self.pop_value()?;
                match (&target, &index) {
                    (Value::Array(arr), Value::Integer(i)) => {
                        let idx = *i as usize;
                        if idx < arr.len() {
                            self.stack.push(arr[idx].clone());
                        } else {
                            return Err(RuntimeError::new("Index out of bounds".to_string()));
                        }
                    }
                    (Value::Map(map), Value::String(key)) => {
                        if let Some(val) = map.get(key) {
                            self.stack.push(val.clone());
                        } else {
                            return Err(RuntimeError::new(format!("Key not found: {}", key)));
                        }
                    }
                    _ => return Err(RuntimeError::new("Invalid index operation".to_string())),
                }
            }
            Instruction::GetField(name) => {
                let obj = self.pop_value()?;
                if let Value::Struct(_, fields) = obj {
                    if let Some(val) = fields.get(name) {
                        self.stack.push(val.clone());
                    } else {
                        return Err(RuntimeError::new(format!("Field not found: {}", name)));
                    }
                } else {
                    return Err(RuntimeError::new("GetField requires struct".to_string()));
                }
            }
            Instruction::TypeOf => {
                let val = self.pop_value()?;
                let type_name = match val {
                    Value::Integer(_) => "int",
                    Value::Float(_) => "float",
                    Value::String(_) => "string",
                    Value::Char(_) => "char",
                    Value::Boolean(_) => "bool",
                    Value::Null => "null",
                    Value::Array(_) => "array",
                    Value::Map(_) => "map",
                    Value::Set(_) => "set",
                    Value::Function(_, _, _, _) => "function",
                    Value::Struct(_, _) => "struct",
                    Value::Enum(_, _) => "enum",
                };
                self.stack.push(Value::String(type_name.to_string()));
            }
            Instruction::Label(_) => {
                // Labels are just markers, no operation
            }
            Instruction::Nop => {
                // No operation
            }
            _ => {
                return Err(RuntimeError::new(format!("Unimplemented instruction: {:?}", inst)));
            }
        }
        Ok(())
    }

    fn pop_value(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(|| RuntimeError::new("Stack underflow".to_string()))
    }

    fn constant_to_value(&self, constant: &Constant) -> Value {
        match constant {
            Constant::Integer(i) => Value::Integer(*i),
            Constant::Float(f) => Value::Float(*f),
            Constant::String(s) => Value::String(s.clone()),
            Constant::Boolean(b) => Value::Boolean(*b),
            Constant::Null => Value::Null,
        }
    }

    fn binary_op<F>(&self, a: Value, b: Value, op: F) -> Result<Value, RuntimeError>
    where
        F: FnOnce(i64, i64) -> i64,
    {
        match (a, b) {
            (Value::Integer(a_val), Value::Integer(b_val)) => Ok(Value::Integer(op(a_val, b_val))),
            _ => Err(RuntimeError::new("Type mismatch in binary operation".to_string())),
        }
    }

    fn compare(&self, a: &Value, b: &Value) -> Result<i32, RuntimeError> {
        match (a, b) {
            (Value::Integer(a_val), Value::Integer(b_val)) => Ok((a_val - b_val).signum() as i32),
            (Value::Float(a_val), Value::Float(b_val)) => {
                Ok(a_val.partial_cmp(b_val).unwrap_or(std::cmp::Ordering::Equal) as i32)
            }
            (Value::String(a_val), Value::String(b_val)) => {
                Ok(a_val.cmp(b_val) as i32)
            }
            _ => Err(RuntimeError::new("Cannot compare values".to_string())),
        }
    }

    fn to_bool(&self, val: &Value) -> Result<bool, RuntimeError> {
        match val {
            Value::Boolean(b) => Ok(*b),
            Value::Integer(i) => Ok(*i != 0),
            Value::Float(f) => Ok(*f != 0.0),
            Value::Null => Ok(false),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Array(arr) => Ok(!arr.is_empty()),
            Value::Map(map) => Ok(!map.is_empty()),
            _ => Ok(true),
        }
    }
}

impl Default for BytecodeVM {
    fn default() -> Self {
        Self::new()
    }
}
