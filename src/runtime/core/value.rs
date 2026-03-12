use crate::parser::ast::{Parameter, Statement};
use std::collections::HashMap;

/// Runtime value representation
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Null,
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Set(Vec<Value>), // Set maintains unique values
    Function(
        String,
        Vec<Parameter>,
        Vec<Statement>,
        HashMap<String, Value>,
    ), // name, params, body, captured_env
    Struct(String, HashMap<String, Value>),
    Enum(String, String),     // enum_name, variant_name
    Result(bool, Box<Value>), // true = Ok(inner), false = Err(inner)
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Value::Map(map) => {
                let items: Vec<String> = map.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Set(set) => {
                let items: Vec<String> = set.iter().map(|v| v.to_string()).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Function(name, _, _, _) => write!(f, "<function {}>", name),
            Value::Struct(name, fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{}({})", name, field_strs.join(", "))
            }
            Value::Enum(name, variant) => write!(f, "{}.{}", name, variant),
            Value::Result(ok, inner) => {
                if *ok {
                    write!(f, "Ok({})", inner)
                } else {
                    write!(f, "Err({})", inner)
                }
            }
        }
    }
}

impl Value {
    /// Check if a value is in a set (for uniqueness checking)
    #[allow(dead_code)]
    pub fn set_contains(set: &[Value], value: &Value) -> bool {
        set.iter().any(|v| v == value)
    }
}
