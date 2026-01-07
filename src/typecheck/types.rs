use crate::parser::ast::Type;
use std::collections::HashMap;

/// Type context for type checking and inference
#[derive(Debug, Clone)]
pub struct TypeContext {
    variables: HashMap<String, Type>,
    functions: HashMap<String, FunctionType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn define_variable(&mut self, name: String, ty: Type) {
        self.variables.insert(name, ty);
    }

    pub fn get_variable(&self, name: &str) -> Option<&Type> {
        self.variables.get(name)
    }

    pub fn define_function(&mut self, name: String, func_type: FunctionType) {
        self.functions.insert(name, func_type);
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionType> {
        self.functions.get(name)
    }
}

/// Type inference result
#[derive(Debug, Clone, PartialEq)]
pub enum InferenceResult {
    Known(Type),
    Unknown,
    Error(String),
}

/// Type compatibility checker
impl Type {
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Int, Type::Int) => true,
            (Type::Float, Type::Float) => true,
            (Type::String, Type::String) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Array(a), Type::Array(b)) => a.is_compatible_with(b),
            (Type::Map(a), Type::Map(b)) => a.is_compatible_with(b),
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => true, // Numeric compatibility
            (Type::Identifier(a), Type::Identifier(b)) => a == b,
            (Type::Generic(_), _) | (_, Type::Generic(_)) => true, // Generics are compatible with anything during inference
            _ => false,
        }
    }

    pub fn unify(&self, other: &Type) -> Option<Type> {
        if self.is_compatible_with(other) {
            // Prefer more specific types
            match (self, other) {
                (Type::Int, Type::Float) | (Type::Float, Type::Int) => Some(Type::Float),
                (a, _) => Some(a.clone()),
            }
        } else {
            None
        }
    }

    pub fn from_runtime_value(value: &crate::runtime::vm::Value) -> Type {
        match value {
            crate::runtime::vm::Value::Integer(_) => Type::Int,
            crate::runtime::vm::Value::Float(_) => Type::Float,
            crate::runtime::vm::Value::String(_) => Type::String,
            crate::runtime::vm::Value::Boolean(_) => Type::Bool,
            crate::runtime::vm::Value::Null => Type::Identifier("null".to_string()),
            crate::runtime::vm::Value::Array(_) => Type::Array(Box::new(Type::Int)), // Default to int array
            crate::runtime::vm::Value::Map(_) => Type::Map(Box::new(Type::String)), // Default to string map
            crate::runtime::vm::Value::Function { .. } => Type::Function {
                params: vec![],
                return_type: Box::new(Type::Int),
            },
        }
    }
}

