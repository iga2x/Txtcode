use crate::runtime::{Value, RuntimeError};
use chrono::{DateTime, Local, TimeZone};

/// Time standard library functions
pub struct TimeLib;

impl TimeLib {
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "now" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(duration) => Ok(Value::Integer(duration.as_secs() as i64)),
                    Err(_) => Err(RuntimeError::new("Failed to get current time".to_string())),
                }
            }
            "sleep" => {
                let ms = if let Some(Value::Integer(m)) = args.first() {
                    *m as u64
                } else if let Some(Value::Float(f)) = args.first() {
                    *f as u64
                } else {
                    return Err(RuntimeError::new("sleep() requires a number argument (milliseconds)".to_string()));
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Null)
            }
            "format_time" | "time_format" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("format_time requires 1 or 2 arguments (timestamp, format?)".to_string()));
                }
                let timestamp = match &args[0] {
                    Value::Integer(ts) => *ts,
                    Value::Float(ts) => *ts as i64,
                    _ => return Err(RuntimeError::new("format_time timestamp must be a number".to_string())),
                };
                let format_str = if args.len() == 2 {
                    match &args[1] {
                        Value::String(f) => f.as_str(),
                        _ => return Err(RuntimeError::new("format_time format must be a string".to_string())),
                    }
                } else {
                    "%Y-%m-%d %H:%M:%S"
                };
                Self::format_time(timestamp, format_str)
            }
            "parse_time" | "time_parse" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("parse_time requires 2 arguments (time_string, format)".to_string()));
                }
                let time_str = match &args[0] {
                    Value::String(s) => s.as_str(),
                    _ => return Err(RuntimeError::new("parse_time time_string must be a string".to_string())),
                };
                let format_str = match &args[1] {
                    Value::String(f) => f.as_str(),
                    _ => return Err(RuntimeError::new("parse_time format must be a string".to_string())),
                };
                Self::parse_time(time_str, format_str)
            }
            _ => Err(RuntimeError::new(format!("Unknown time function: {}", name))),
        }
    }

    fn format_time(timestamp: i64, format: &str) -> Result<Value, RuntimeError> {
        let dt = match Local.timestamp_opt(timestamp, 0) {
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(dt, _) => dt,
            chrono::LocalResult::None => {
                return Err(RuntimeError::new("Invalid timestamp".to_string()));
            }
        };
        Ok(Value::String(dt.format(format).to_string()))
    }

    fn parse_time(time_str: &str, format: &str) -> Result<Value, RuntimeError> {
        let dt = DateTime::parse_from_str(time_str, format)
            .map_err(|e| RuntimeError::new(format!("Failed to parse time: {}", e)))?;
        Ok(Value::Integer(dt.timestamp()))
    }
}

