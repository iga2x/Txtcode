use crate::runtime::{RuntimeError, Value};
use chrono::{DateTime, Local, TimeZone};

/// Time standard library functions
pub struct TimeLib;

impl TimeLib {
    pub fn call_function(
        name: &str,
        args: &[Value],
        time_override: Option<std::time::SystemTime>,
    ) -> Result<Value, RuntimeError> {
        match name {
            "now" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let t = time_override.unwrap_or_else(SystemTime::now);
                match t.duration_since(UNIX_EPOCH) {
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
                    return Err(RuntimeError::new(
                        "sleep() requires a number argument (milliseconds)".to_string(),
                    ));
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Null)
            }
            "format_time" | "time_format" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RuntimeError::new(
                        "format_time requires 1 or 2 arguments (timestamp, format?)".to_string(),
                    ));
                }
                let timestamp = match &args[0] {
                    Value::Integer(ts) => *ts,
                    Value::Float(ts) => *ts as i64,
                    _ => {
                        return Err(RuntimeError::new(
                            "format_time timestamp must be a number".to_string(),
                        ))
                    }
                };
                let format_str = if args.len() == 2 {
                    match &args[1] {
                        Value::String(f) => f.as_str(),
                        _ => {
                            return Err(RuntimeError::new(
                                "format_time format must be a string".to_string(),
                            ))
                        }
                    }
                } else {
                    "%Y-%m-%d %H:%M:%S"
                };
                Self::format_time(timestamp, format_str)
            }
            "parse_time" | "time_parse" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "parse_time requires 2 arguments (time_string, format)".to_string(),
                    ));
                }
                let time_str = match &args[0] {
                    Value::String(s) => s.as_str(),
                    _ => {
                        return Err(RuntimeError::new(
                            "parse_time time_string must be a string".to_string(),
                        ))
                    }
                };
                let format_str = match &args[1] {
                    Value::String(f) => f.as_str(),
                    _ => {
                        return Err(RuntimeError::new(
                            "parse_time format must be a string".to_string(),
                        ))
                    }
                };
                Self::parse_time(time_str, format_str)
            }
            "now_utc" => {
                use chrono::Utc;
                Ok(Value::String(Utc::now().to_rfc3339()))
            }
            "now_local" => {
                Ok(Value::String(Local::now().to_rfc3339()))
            }
            "parse_datetime" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "parse_datetime requires 2 arguments (datetime_string, format)".to_string(),
                    ));
                }
                let s = match &args[0] {
                    Value::String(s) => s.as_str(),
                    _ => return Err(RuntimeError::new("parse_datetime: first argument must be a string".to_string())),
                };
                let fmt = match &args[1] {
                    Value::String(f) => f.as_str(),
                    _ => return Err(RuntimeError::new("parse_datetime: second argument must be a string".to_string())),
                };
                Self::parse_time(s, fmt)
            }
            "format_datetime" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "format_datetime requires 2 or 3 arguments (timestamp, format, tz?)".to_string(),
                    ));
                }
                let ts = match &args[0] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("format_datetime: timestamp must be a number".to_string())),
                };
                let fmt = match &args[1] {
                    Value::String(f) => f.clone(),
                    _ => return Err(RuntimeError::new("format_datetime: format must be a string".to_string())),
                };
                let tz = if args.len() == 3 {
                    match &args[2] {
                        Value::String(z) => z.as_str(),
                        _ => return Err(RuntimeError::new("format_datetime: tz must be a string".to_string())),
                    }
                } else {
                    "local"
                };
                match tz.to_lowercase().as_str() {
                    "utc" => {
                        use chrono::{TimeZone, Utc};
                        match Utc.timestamp_opt(ts, 0) {
                            chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => {
                                Ok(Value::String(dt.format(&fmt).to_string()))
                            }
                            chrono::LocalResult::None => Err(RuntimeError::new("format_datetime: invalid timestamp".to_string())),
                        }
                    }
                    _ => Self::format_time(ts, &fmt),
                }
            }
            "datetime_add" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "datetime_add requires 3 arguments (timestamp, amount, unit)".to_string(),
                    ));
                }
                let ts = match &args[0] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("datetime_add: timestamp must be a number".to_string())),
                };
                let amount = match &args[1] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("datetime_add: amount must be a number".to_string())),
                };
                let unit = match &args[2] {
                    Value::String(s) => s.as_str(),
                    _ => return Err(RuntimeError::new("datetime_add: unit must be a string".to_string())),
                };
                let delta_secs: i64 = match unit {
                    "seconds" | "second" => amount,
                    "minutes" | "minute" => amount * 60,
                    "hours" | "hour" => amount * 3600,
                    "days" | "day" => amount * 86400,
                    "weeks" | "week" => amount * 604800,
                    other => return Err(RuntimeError::new(format!("datetime_add: unknown unit '{}' (use seconds/minutes/hours/days/weeks)", other))),
                };
                Ok(Value::Integer(ts + delta_secs))
            }
            "datetime_diff" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(
                        "datetime_diff requires 3 arguments (ts1, ts2, unit)".to_string(),
                    ));
                }
                let ts1 = match &args[0] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("datetime_diff: ts1 must be a number".to_string())),
                };
                let ts2 = match &args[1] {
                    Value::Integer(n) => *n,
                    Value::Float(f) => *f as i64,
                    _ => return Err(RuntimeError::new("datetime_diff: ts2 must be a number".to_string())),
                };
                let unit = match &args[2] {
                    Value::String(s) => s.as_str(),
                    _ => return Err(RuntimeError::new("datetime_diff: unit must be a string".to_string())),
                };
                let diff_secs = ts1 - ts2;
                let result = match unit {
                    "seconds" | "second" => diff_secs,
                    "minutes" | "minute" => diff_secs / 60,
                    "hours" | "hour" => diff_secs / 3600,
                    "days" | "day" => diff_secs / 86400,
                    "weeks" | "week" => diff_secs / 604800,
                    other => return Err(RuntimeError::new(format!("datetime_diff: unknown unit '{}' (use seconds/minutes/hours/days/weeks)", other))),
                };
                Ok(Value::Integer(result))
            }
            _ => Err(RuntimeError::new(format!(
                "Unknown time function: {}",
                name
            ))),
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
