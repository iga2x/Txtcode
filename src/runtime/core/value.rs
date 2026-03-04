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
    Function(String, Vec<Parameter>, Vec<Statement>, HashMap<String, Value>), // name, params, body, captured_env
    Struct(String, HashMap<String, Value>),
    Enum(String, String), // enum_name, variant_name
}

impl Value {
    pub fn to_string(&self) -> String {
        match self {
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Char(c) => format!("'{}'", c),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Map(map) => {
                let items: Vec<String> = map.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Set(set) => {
                let items: Vec<String> = set.iter().map(|v| v.to_string()).collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Function(name, _, _, _) => format!("<function {}>", name),
            Value::Struct(name, fields) => {
                let field_strs: Vec<String> = fields.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{}({})", name, field_strs.join(", "))
            }
            Value::Enum(name, variant) => format!("{}.{}", name, variant),
        }
    }
    
    /// Check if a value is in a set (for uniqueness checking)
    #[allow(dead_code)]
    pub fn set_contains(set: &[Value], value: &Value) -> bool {
        set.iter().any(|v| v == value)
    }
}

