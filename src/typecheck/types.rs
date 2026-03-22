use std::collections::HashMap;
use std::collections::HashSet;

/// Type system representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// No annotation provided — skip type enforcement, accept any value.
    /// This replaces the old incorrect default of Type::Int for unannotated params.
    Unknown,
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
    Nullable(Box<Type>), // T? — type that can also be null
    Null,                // the null type itself
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            // Unknown means "unannotated" — always compatible with anything
            (Type::Unknown, _) | (_, Type::Unknown) => true,
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
            // Nullable: null is compatible with T?, T is compatible with T?
            (Type::Null, Type::Nullable(_)) => true,
            (inner, Type::Nullable(outer)) => inner.is_compatible_with(outer),
            (Type::Nullable(inner), other) => inner.is_compatible_with(other),
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
    /// Names of generic type parameters, e.g. `["T", "U"]`
    pub generic_params: Vec<String>,
    /// Constraint bounds per type var, e.g. `{"T": "Comparable"}`
    pub generic_constraints: HashMap<String, String>,
}

/// Allowed types for each built-in constraint
pub fn constraint_allowed_types(constraint: &str) -> HashSet<&'static str> {
    match constraint {
        "Comparable" => ["Int", "Float", "String", "Bool", "Char"].iter().copied().collect(),
        "Numeric"    => ["Int", "Float"].iter().copied().collect(),
        "Printable"  => ["Int", "Float", "String", "Bool", "Char", "Null"].iter().copied().collect(),
        _ => HashSet::new(),
    }
}

/// Return the canonical constraint-check name for a Type
pub fn type_constraint_name(ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Int   => Some("Int"),
        Type::Float => Some("Float"),
        Type::String => Some("String"),
        Type::Bool  => Some("Bool"),
        Type::Char  => Some("Char"),
        Type::Null  => Some("Null"),
        _ => None, // Array, Map, etc. — never satisfy constraints
    }
}

/// Type context for tracking variable and function types
#[derive(Clone)]
pub struct TypeContext {
    variables: HashMap<String, Type>,
    functions: HashMap<String, FunctionType>,
    /// K.3: Enum variant registry — enum_name → [variant1, variant2, ...]
    pub enum_variants: HashMap<String, Vec<String>>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            enum_variants: HashMap::new(),
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

    // K.3: Enum variant registry
    pub fn define_enum(&mut self, name: String, variants: Vec<String>) {
        self.enum_variants.insert(name, variants);
    }

    pub fn get_enum_variants(&self, name: &str) -> Option<&Vec<String>> {
        self.enum_variants.get(name)
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
