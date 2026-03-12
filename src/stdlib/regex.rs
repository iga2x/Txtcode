use crate::runtime::{RuntimeError, Value};
use regex::Regex;
use std::collections::HashMap;

/// Regular expression library
pub struct RegexLib;

impl RegexLib {
    /// Call a regex library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "regex_match" | "regex_test" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "regex_match requires 2 arguments (pattern, text)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(pattern), Value::String(text)) => {
                        Self::regex_match(pattern, text)
                    }
                    _ => Err(RuntimeError::new(
                        "regex_match requires string arguments".to_string(),
                    )),
                }
            }
            "regex_find" | "regex_search" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "regex_find requires 2 arguments (pattern, text)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(pattern), Value::String(text)) => {
                        Self::regex_find(pattern, text)
                    }
                    _ => Err(RuntimeError::new(
                        "regex_find requires string arguments".to_string(),
                    )),
                }
            }
            "regex_find_all" | "regex_findall" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "regex_find_all requires 2 arguments (pattern, text)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(pattern), Value::String(text)) => {
                        Self::regex_find_all(pattern, text)
                    }
                    _ => Err(RuntimeError::new(
                        "regex_find_all requires string arguments".to_string(),
                    )),
                }
            }
            "regex_replace" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "regex_replace requires 3 arguments (pattern, text, replacement)"
                            .to_string(),
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(pattern), Value::String(text), Value::String(replacement)) => {
                        Self::regex_replace(pattern, text, replacement)
                    }
                    _ => Err(RuntimeError::new(
                        "regex_replace requires string arguments".to_string(),
                    )),
                }
            }
            "regex_replace_all" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "regex_replace_all requires 3 arguments (pattern, text, replacement)"
                            .to_string(),
                    ));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(pattern), Value::String(text), Value::String(replacement)) => {
                        Self::regex_replace_all(pattern, text, replacement)
                    }
                    _ => Err(RuntimeError::new(
                        "regex_replace_all requires string arguments".to_string(),
                    )),
                }
            }
            "regex_split" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "regex_split requires 2 arguments (pattern, text)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(pattern), Value::String(text)) => {
                        Self::regex_split(pattern, text)
                    }
                    _ => Err(RuntimeError::new(
                        "regex_split requires string arguments".to_string(),
                    )),
                }
            }
            _ => Err(RuntimeError::new(format!(
                "Unknown regex function: {}",
                name
            ))),
        }
    }

    fn regex_match(pattern: &str, text: &str) -> Result<Value, RuntimeError> {
        let re = Regex::new(pattern)
            .map_err(|e| RuntimeError::new(format!("Invalid regex pattern: {}", e)))?;
        Ok(Value::Boolean(re.is_match(text)))
    }

    fn regex_find(pattern: &str, text: &str) -> Result<Value, RuntimeError> {
        let re = Regex::new(pattern)
            .map_err(|e| RuntimeError::new(format!("Invalid regex pattern: {}", e)))?;

        if let Some(captures) = re.captures(text) {
            if let Some(matched) = captures.get(0) {
                let mut result = HashMap::new();
                result.insert(
                    "match".to_string(),
                    Value::String(matched.as_str().to_string()),
                );
                result.insert("start".to_string(), Value::Integer(matched.start() as i64));
                result.insert("end".to_string(), Value::Integer(matched.end() as i64));

                // Add capture groups
                let mut groups = Vec::new();
                for i in 1..captures.len() {
                    if let Some(group) = captures.get(i) {
                        groups.push(Value::String(group.as_str().to_string()));
                    } else {
                        groups.push(Value::Null);
                    }
                }
                result.insert("groups".to_string(), Value::Array(groups));

                Ok(Value::Map(result))
            } else {
                Ok(Value::Null)
            }
        } else {
            Ok(Value::Null)
        }
    }

    fn regex_find_all(pattern: &str, text: &str) -> Result<Value, RuntimeError> {
        let re = Regex::new(pattern)
            .map_err(|e| RuntimeError::new(format!("Invalid regex pattern: {}", e)))?;

        let matches: Vec<Value> = re
            .find_iter(text)
            .map(|m| {
                let mut result = HashMap::new();
                result.insert("match".to_string(), Value::String(m.as_str().to_string()));
                result.insert("start".to_string(), Value::Integer(m.start() as i64));
                result.insert("end".to_string(), Value::Integer(m.end() as i64));
                Value::Map(result)
            })
            .collect();

        Ok(Value::Array(matches))
    }

    fn regex_replace(pattern: &str, text: &str, replacement: &str) -> Result<Value, RuntimeError> {
        let re = Regex::new(pattern)
            .map_err(|e| RuntimeError::new(format!("Invalid regex pattern: {}", e)))?;
        Ok(Value::String(re.replace(text, replacement).to_string()))
    }

    fn regex_replace_all(
        pattern: &str,
        text: &str,
        replacement: &str,
    ) -> Result<Value, RuntimeError> {
        let re = Regex::new(pattern)
            .map_err(|e| RuntimeError::new(format!("Invalid regex pattern: {}", e)))?;
        Ok(Value::String(re.replace_all(text, replacement).to_string()))
    }

    fn regex_split(pattern: &str, text: &str) -> Result<Value, RuntimeError> {
        let re = Regex::new(pattern)
            .map_err(|e| RuntimeError::new(format!("Invalid regex pattern: {}", e)))?;
        let parts: Vec<Value> = re
            .split(text)
            .map(|s| Value::String(s.to_string()))
            .collect();
        Ok(Value::Array(parts))
    }
}
