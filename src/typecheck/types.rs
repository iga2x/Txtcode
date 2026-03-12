use std::collections::HashMap;

/// Type system representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    String,
    Char,
    Bool,
    Array(Box<Type>),
    Map(Box<Type>),
    Set(Box<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Future(Box<Type>), // Future<T> - async type
    Identifier(String),
    Generic(String),
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::String, Type::String)
            | (Type::Char, Type::Char)
            | (Type::Bool, Type::Bool) => true,
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => true,
            (Type::Char, Type::String) | (Type::String, Type::Char) => true, // Char can convert to String
            (Type::Array(a), Type::Array(b)) => a.is_compatible_with(b),
            (Type::Map(a), Type::Map(b)) => a.is_compatible_with(b),
            (Type::Set(a), Type::Set(b)) => a.is_compatible_with(b),
            (Type::Future(a), Type::Future(b)) => a.is_compatible_with(b),
            (Type::Identifier(a), Type::Identifier(b)) => a == b,
            (Type::Generic(a), Type::Generic(b)) => a == b,
            _ => false,
        }
    }
}

/// Type inference result
#[derive(Debug, Clone, PartialEq)]
pub enum InferenceResult {
    Known(Type),
    Unknown,
    Error(String),
}

/// Function type definition
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

/// Type context for tracking variable and function types
#[derive(Clone)]
pub struct TypeContext {
    variables: HashMap<String, Type>,
    functions: HashMap<String, FunctionType>,
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

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Type substitution for generics
pub struct TypeSubstitution {
    mappings: HashMap<String, Type>,
}

impl TypeSubstitution {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn add(&mut self, generic: String, concrete: Type) {
        self.mappings.insert(generic, concrete);
    }

    pub fn get(&self, generic: &str) -> Option<&Type> {
        self.mappings.get(generic)
    }

    /// Substitute generic types in a type with concrete types
    pub fn substitute(&self, ty: &Type) -> Type {
        match ty {
            Type::Generic(name) => self
                .mappings
                .get(name)
                .cloned()
                .unwrap_or_else(|| Type::Generic(name.clone())),
            Type::Array(inner) => Type::Array(Box::new(self.substitute(inner))),
            Type::Map(inner) => Type::Map(Box::new(self.substitute(inner))),
            Type::Set(inner) => Type::Set(Box::new(self.substitute(inner))),
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(|p| self.substitute(p)).collect(),
                return_type: Box::new(self.substitute(return_type)),
            },
            _ => ty.clone(),
        }
    }
}

impl Default for TypeSubstitution {
    fn default() -> Self {
        Self::new()
    }
}
