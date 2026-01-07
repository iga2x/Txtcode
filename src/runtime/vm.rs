use crate::parser::ast::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    #[serde(skip)]
    Function {
        name: String,
        params: Vec<Parameter>,
        body: Vec<Statement>,
        closure: Rc<Environment>,
    },
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::Integer(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Boolean(_) => "bool",
            Value::Null => "null",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
            Value::Function { .. } => "function",
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Map(map) => {
                let items: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Function { name, .. } => format!("<function {}>", name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<Rc<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Rc<Environment>) -> Self {
        Self {
            values: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else if let Some(_parent) = &self.parent {
            // Try to assign in parent scope
            // Note: This is a simplified version. In a real implementation,
            // we'd need mutable references to parent environments.
            Err(RuntimeError {
                message: format!("Undefined variable: {}", name),
            })
        } else {
            Err(RuntimeError {
                message: format!("Undefined variable: {}", name),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

pub struct VirtualMachine {
    environment: Rc<Environment>,
    step_count: u64,
    max_steps: u64,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            environment: Rc::new(Environment::new()),
            step_count: 0,
            max_steps: 1_000_000, // Default: 1 million steps max
        }
    }

    pub fn with_max_steps(max_steps: u64) -> Self {
        Self {
            environment: Rc::new(Environment::new()),
            step_count: 0,
            max_steps,
        }
    }

    fn check_step_limit(&mut self) -> Result<(), RuntimeError> {
        self.step_count += 1;
        if self.step_count > self.max_steps {
            Err(RuntimeError {
                message: format!(
                    "Execution limit exceeded ({} steps). Possible infinite loop detected.",
                    self.max_steps
                ),
            })
        } else {
            Ok(())
        }
    }

    pub fn interpret(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        let mut last_value = Value::Null;
        self.step_count = 0; // Reset step counter for new program

        for statement in &program.statements {
            self.check_step_limit()?;
            last_value = self.execute_statement(statement)?;
        }

        Ok(last_value)
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<Value, RuntimeError> {
        match statement {
            Statement::Expression(expr) => self.evaluate_expression(expr),
            Statement::Assignment { name, value, .. } => {
                let val = self.evaluate_expression(value)?;
                // In a real implementation, we'd need mutable access to environment
                // For now, we'll create a new environment
                let mut new_env = Environment::new();
                new_env.define(name.clone(), val.clone());
                Ok(val)
            }
            Statement::CompoundAssignment { name, op, value, .. } => {
                // Get current value of variable
                let current_val = self.environment
                    .get(name)
                    .ok_or_else(|| RuntimeError {
                        message: format!("Undefined variable: {}", name),
                    })?;
                // Evaluate the right-hand side
                let right_val = self.evaluate_expression(value)?;
                // Perform the operation
                let result = self.binary_operation(&current_val, op, &right_val)?;
                // In a real implementation, we'd update the environment
                // For now, we'll create a new environment
                let mut new_env = Environment::new();
                new_env.define(name.clone(), result.clone());
                Ok(result)
            }
            Statement::FunctionDef {
                name,
                params,
                body,
                ..
            } => {
                let func = Value::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.environment),
                };
                // In a real implementation, we'd store this in the environment
                Ok(func)
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.evaluate_expression(expr)
                } else {
                    Ok(Value::Null)
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let cond_value = self.evaluate_expression(condition)?;
                if self.is_truthy(&cond_value) {
                    self.execute_block(then_branch)
                } else {
                    for (elseif_cond, elseif_body) in else_if_branches {
                        let elseif_value = self.evaluate_expression(elseif_cond)?;
                        if self.is_truthy(&elseif_value) {
                            return self.execute_block(elseif_body);
                        }
                    }
                    // No elseif matched, execute else branch if present
                    if let Some(else_body) = else_branch {
                        self.execute_block(else_body)
                    } else {
                        Ok(Value::Null)
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                let mut iterations = 0;
                const MAX_ITERATIONS: u64 = 100_000; // Prevent infinite loops
                
                loop {
                    self.check_step_limit()?;
                    iterations += 1;
                    if iterations > MAX_ITERATIONS {
                        return Err(RuntimeError {
                            message: format!(
                                "While loop exceeded maximum iterations ({}). Possible infinite loop.",
                                MAX_ITERATIONS
                            ),
                        });
                    }
                    
                    let cond_value = self.evaluate_expression(condition)?;
                    if !self.is_truthy(&cond_value) {
                        break;
                    }
                    // Execute body statements
                    for stmt in body {
                        self.check_step_limit()?;
                        self.execute_statement(stmt)?;
                    }
                }
                Ok(Value::Null)
            }
            Statement::For {
                variable: _variable,
                iterable,
                body,
                ..
            } => {
                let iter_value = self.evaluate_expression(iterable)?;
                match iter_value {
                    Value::Array(arr) => {
                        if arr.len() > 100_000 {
                            return Err(RuntimeError {
                                message: format!(
                                    "For loop array too large ({} items). Maximum allowed: 100,000",
                                    arr.len()
                                ),
                            });
                        }
                        for _item in arr {
                            self.check_step_limit()?;
                            // In a real implementation, we'd create a new scope
                            // and define the variable there
                            for stmt in body {
                                self.check_step_limit()?;
                                self.execute_statement(stmt)?;
                            }
                        }
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError {
                        message: "For loop requires an iterable value".to_string(),
                    }),
                }
            }
            Statement::Repeat { count, body, .. } => {
                let count_value = self.evaluate_expression(count)?;
                let n = match count_value {
                    Value::Integer(n) => {
                        if n < 0 {
                            return Err(RuntimeError {
                                message: "Repeat count cannot be negative".to_string(),
                            });
                        }
                        if n > 100_000 {
                            return Err(RuntimeError {
                                message: format!(
                                    "Repeat count too large ({}). Maximum allowed: 100,000",
                                    n
                                ),
                            });
                        }
                        n as u64
                    }
                    _ => {
                        return Err(RuntimeError {
                            message: "Repeat count must be an integer".to_string(),
                        });
                    }
                };

                for _ in 0..n {
                    self.check_step_limit()?;
                    for stmt in body {
                        self.check_step_limit()?;
                        self.execute_statement(stmt)?;
                    }
                }
                Ok(Value::Null)
            }
            Statement::Break { .. } => Ok(Value::Null), // Simplified
            Statement::Continue { .. } => Ok(Value::Null), // Simplified
            Statement::Match { value, cases, default, .. } => {
                let match_value = self.evaluate_expression(value)?;
                for case in cases {
                    if self.match_pattern(&match_value, &case.pattern)? {
                        if let Some(guard) = &case.guard {
                            let guard_value = self.evaluate_expression(guard)?;
                            if !self.is_truthy(&guard_value) {
                                continue;
                            }
                        }
                        return self.execute_block(&case.body);
                    }
                }
                if let Some(default_body) = default {
                    self.execute_block(default_body)
                } else {
                    Ok(Value::Null)
                }
            }
            Statement::Try { body, catch, .. } => {
                match self.execute_block(body) {
                    Ok(val) => Ok(val),
                    Err(e) => {
                        if let Some((_error_var, catch_body)) = catch {
                            // In a real implementation, we'd define error_var in a new scope
                            self.execute_block(catch_body)
                        } else {
                            Err(e)
                        }
                    }
                }
            }
            Statement::Import { .. } => {
                // Import handling would be implemented here
                Ok(Value::Null)
            }
            Statement::Assert { condition, message, .. } => {
                let cond_value = self.evaluate_expression(condition)?;
                if !self.is_truthy(&cond_value) {
                    let error_msg = if let Some(msg_expr) = message {
                        self.evaluate_expression(msg_expr)?.to_string()
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(RuntimeError {
                        message: error_msg,
                    });
                }
                Ok(Value::Null)
            }
        }
    }

    fn execute_block(&mut self, statements: &[Statement]) -> Result<Value, RuntimeError> {
        let mut last_value = Value::Null;
        for statement in statements {
            last_value = self.execute_statement(statement)?;
        }
        Ok(last_value)
    }

    fn evaluate_expression(&self, expr: &Expression) -> Result<Value, RuntimeError> {
        match expr {
            Expression::Literal(lit) => Ok(self.literal_to_value(lit)),
            Expression::Identifier(name) => {
                self.environment
                    .get(name)
                    .ok_or_else(|| RuntimeError {
                        message: format!("Undefined variable: {}", name),
                    })
            }
            Expression::BinaryOp { left, op, right, .. } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                self.binary_operation(&left_val, op, &right_val)
            }
            Expression::UnaryOp { op, operand, .. } => {
                let operand_val = self.evaluate_expression(operand)?;
                self.unary_operation(op, &operand_val)
            }
            Expression::FunctionCall { name, arguments, .. } => {
                let args: Result<Vec<Value>, RuntimeError> =
                    arguments.iter().map(|arg| self.evaluate_expression(arg)).collect();
                let args = args?;
                self.call_function(name, &args)
            }
            Expression::Array { elements, .. } => {
                let values: Result<Vec<Value>, RuntimeError> =
                    elements.iter().map(|e| self.evaluate_expression(e)).collect();
                Ok(Value::Array(values?))
            }
            Expression::Map { entries, .. } => {
                let mut map = HashMap::new();
                for (key, value_expr) in entries {
                    let value = self.evaluate_expression(value_expr)?;
                    map.insert(key.clone(), value);
                }
                Ok(Value::Map(map))
            }
            Expression::Index { target, index, .. } => {
                let target_val = self.evaluate_expression(target)?;
                let index_val = self.evaluate_expression(index)?;
                self.index_value(&target_val, &index_val)
            }
            Expression::Slice { target, start, end, .. } => {
                let target_val = self.evaluate_expression(target)?;
                let start_val = if let Some(start_expr) = start {
                    Some(self.evaluate_expression(start_expr)?)
                } else {
                    None
                };
                let end_val = if let Some(end_expr) = end {
                    Some(self.evaluate_expression(end_expr)?)
                } else {
                    None
                };
                self.slice_value(&target_val, start_val.as_ref(), end_val.as_ref())
            }
            Expression::Member { target, member, .. } => {
                let target_val = self.evaluate_expression(target)?;
                self.get_member(&target_val, member)
            }
            Expression::Lambda { params: _params, body: _body, .. } => {
                // Lambda functions would be implemented here
                Ok(Value::Null) // Simplified
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                let cond_value = self.evaluate_expression(condition)?;
                if self.is_truthy(&cond_value) {
                    self.evaluate_expression(true_expr)
                } else {
                    self.evaluate_expression(false_expr)
                }
            }
        }
    }

    fn literal_to_value(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Integer(n) => Value::Integer(*n),
            Literal::Float(n) => Value::Float(*n),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Boolean(b) => Value::Boolean(*b),
            Literal::Null => Value::Null,
        }
    }

    fn binary_operation(
        &self,
        left: &Value,
        op: &BinaryOperator,
        right: &Value,
    ) -> Result<Value, RuntimeError> {
        match op {
            BinaryOperator::Add => self.add_values(left, right),
            BinaryOperator::Subtract => self.subtract_values(left, right),
            BinaryOperator::Multiply => self.multiply_values(left, right),
            BinaryOperator::Divide => self.divide_values(left, right),
            BinaryOperator::Modulo => self.modulo_values(left, right),
            BinaryOperator::Power => self.power_values(left, right),
            BinaryOperator::Equal => Ok(Value::Boolean(self.values_equal(left, right))),
            BinaryOperator::NotEqual => Ok(Value::Boolean(!self.values_equal(left, right))),
            BinaryOperator::Less => self.compare_values(left, right, |a, b| a < b),
            BinaryOperator::Greater => self.compare_values(left, right, |a, b| a > b),
            BinaryOperator::LessEqual => self.compare_values(left, right, |a, b| a <= b),
            BinaryOperator::GreaterEqual => self.compare_values(left, right, |a, b| a >= b),
            BinaryOperator::And => Ok(Value::Boolean(self.is_truthy(left) && self.is_truthy(right))),
            BinaryOperator::Or => Ok(Value::Boolean(self.is_truthy(left) || self.is_truthy(right))),
            BinaryOperator::BitAnd => self.bitwise_and(left, right),
            BinaryOperator::BitOr => self.bitwise_or(left, right),
            BinaryOperator::BitXor => self.bitwise_xor(left, right),
            BinaryOperator::LeftShift => self.left_shift(left, right),
            BinaryOperator::RightShift => self.right_shift(left, right),
            BinaryOperator::Arrow => {
                // Arrow operator for function calls
                self.call_function_with_value(left, right)
            }
        }
    }

    fn unary_operation(&self, op: &UnaryOperator, operand: &Value) -> Result<Value, RuntimeError> {
        match op {
            UnaryOperator::Not => Ok(Value::Boolean(!self.is_truthy(operand))),
            UnaryOperator::Minus => self.negate_value(operand),
            UnaryOperator::BitNot => self.bitwise_not(operand),
        }
    }

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
                    Err(RuntimeError {
                        message: "Division by zero".to_string(),
                    })
                } else {
                    Ok(Value::Float(*a as f64 / *b as f64))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError {
                        message: "Division by zero".to_string(),
                    })
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (Value::Integer(a), Value::Float(b)) => {
                if *b == 0.0 {
                    Err(RuntimeError {
                        message: "Division by zero".to_string(),
                    })
                } else {
                    Ok(Value::Float(*a as f64 / b))
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                if *b == 0 {
                    Err(RuntimeError {
                        message: "Division by zero".to_string(),
                    })
                } else {
                    Ok(Value::Float(a / *b as f64))
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
                    Err(RuntimeError {
                        message: "Modulo by zero".to_string(),
                    })
                } else {
                    Ok(Value::Integer(a % b))
                }
            }
            _ => Err(RuntimeError {
                message: format!("Cannot compute modulo for {} and {}", left.type_name(), right.type_name()),
            }),
        }
    }

    fn power_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                Ok(Value::Float((*a as f64).powf(*b as f64)))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.powf(*b as f64))),
            _ => Err(RuntimeError {
                message: format!("Cannot compute power for {} and {}", left.type_name(), right.type_name()),
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

    fn negate_value(&self, operand: &Value) -> Result<Value, RuntimeError> {
        match operand {
            Value::Integer(n) => Ok(Value::Integer(-n)),
            Value::Float(n) => Ok(Value::Float(-n)),
            _ => Err(RuntimeError {
                message: format!("Cannot negate {}", operand.type_name()),
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

    fn slice_value(&self, target: &Value, start: Option<&Value>, end: Option<&Value>) -> Result<Value, RuntimeError> {
        match target {
            Value::Array(arr) => {
                let start_idx = if let Some(Value::Integer(s)) = start {
                    let idx = *s as i64;
                    if idx < 0 {
                        (arr.len() as i64 + idx).max(0) as usize
                    } else {
                        idx.min(arr.len() as i64) as usize
                    }
                } else {
                    0
                };
                let end_idx = if let Some(Value::Integer(e)) = end {
                    let idx = *e as i64;
                    if idx < 0 {
                        (arr.len() as i64 + idx).max(0) as usize
                    } else {
                        idx.min(arr.len() as i64) as usize
                    }
                } else {
                    arr.len()
                };
                if start_idx > end_idx {
                    return Err(RuntimeError {
                        message: format!("Invalid slice range: {} to {}", start_idx, end_idx),
                    });
                }
                Ok(Value::Array(arr[start_idx..end_idx].to_vec()))
            }
            Value::String(s) => {
                let start_idx = if let Some(Value::Integer(st)) = start {
                    let idx = *st as i64;
                    if idx < 0 {
                        (s.len() as i64 + idx).max(0) as usize
                    } else {
                        idx.min(s.len() as i64) as usize
                    }
                } else {
                    0
                };
                let end_idx = if let Some(Value::Integer(e)) = end {
                    let idx = *e as i64;
                    if idx < 0 {
                        (s.len() as i64 + idx).max(0) as usize
                    } else {
                        idx.min(s.len() as i64) as usize
                    }
                } else {
                    s.len()
                };
                if start_idx > end_idx {
                    return Err(RuntimeError {
                        message: format!("Invalid slice range: {} to {}", start_idx, end_idx),
                    });
                }
                Ok(Value::String(s[start_idx..end_idx].to_string()))
            }
            _ => Err(RuntimeError {
                message: "Slice operation requires an array or string".to_string(),
            }),
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
                map.get(key)
                    .cloned()
                    .ok_or_else(|| RuntimeError {
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
                map.get(member)
                    .cloned()
                    .ok_or_else(|| RuntimeError {
                        message: format!("Member '{}' not found", member),
                    })
            }
            _ => Err(RuntimeError {
                message: format!("Cannot access member '{}' on {}", member, target.type_name()),
            }),
        }
    }

    fn call_function(&self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        // Built-in functions
        match name {
            "print" => {
                // Print all arguments separated by spaces (like Python)
                let output: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                println!("{}", output.join(" "));
                Ok(Value::Null)
            }
            _ => {
                // Try standard library first
                if let Ok(result) = crate::stdlib::StdLib::call_function(name, args) {
                    return Ok(result);
                }
                
                // Try to get function from environment
                if let Some(Value::Function { params: _params, body: _body, closure: _closure, name: _ }) = self.environment.get(name) {
                    // In a real implementation, we'd execute the function body
                    // with the arguments bound to parameters in a new environment
                    Ok(Value::Null) // Simplified
                } else {
                    Err(RuntimeError {
                        message: format!("Function '{}' not found", name),
                    })
                }
            }
        }
    }

    fn call_function_with_value(&self, callee: &Value, _args: &Value) -> Result<Value, RuntimeError> {
        match callee {
            Value::Function { .. } => {
                // In a real implementation, we'd extract arguments from the right value
                // and call the function
                Ok(Value::Null) // Simplified
            }
            _ => Err(RuntimeError {
                message: "Arrow operator requires a function".to_string(),
            }),
        }
    }

    fn match_pattern(&self, value: &Value, pattern: &Pattern) -> Result<bool, RuntimeError> {
        match pattern {
            Pattern::Literal(expr) => {
                // Evaluate the expression to get the literal value
                let lit_value = self.evaluate_expression(expr)?;
                Ok(self.values_equal(value, &lit_value))
            }
            Pattern::Identifier(_name) => {
                // In a real implementation, we'd bind the value to the identifier
                Ok(true) // Simplified
            }
            Pattern::Wildcard => Ok(true),
        }
    }
}

// Span helper for expressions - implemented in parser/ast.rs

