use crate::runtime::{RuntimeError, Value};
use std::sync::Arc;
use std::path::{Path, PathBuf};

/// Path manipulation library
pub struct PathLib;

impl PathLib {
    /// Call a path library function
    pub fn call_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "path_join" | "path_combine" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "path_join requires at least 1 argument".to_string(),
                    ));
                }
                let paths: Vec<String> = args
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(s.to_string()),
                        _ => Err(RuntimeError::new(
                            "path_join requires string arguments".to_string(),
                        )),
                    })
                    .collect::<Result<Vec<String>, RuntimeError>>()?;
                Self::path_join(&paths)
            }
            "path_dir" | "path_directory" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_dir requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Self::path_dir(path),
                    _ => Err(RuntimeError::new(
                        "path_dir requires a string argument".to_string(),
                    )),
                }
            }
            "path_base" | "path_filename" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_base requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Self::path_base(path),
                    _ => Err(RuntimeError::new(
                        "path_base requires a string argument".to_string(),
                    )),
                }
            }
            "path_ext" | "path_extension" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_ext requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Self::path_ext(path),
                    _ => Err(RuntimeError::new(
                        "path_ext requires a string argument".to_string(),
                    )),
                }
            }
            "path_stem" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_stem requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Self::path_stem(path),
                    _ => Err(RuntimeError::new(
                        "path_stem requires a string argument".to_string(),
                    )),
                }
            }
            "path_abs" | "path_absolute" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_abs requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Self::path_abs(path),
                    _ => Err(RuntimeError::new(
                        "path_abs requires a string argument".to_string(),
                    )),
                }
            }
            "path_norm" | "path_normalize" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_norm requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Self::path_norm(path),
                    _ => Err(RuntimeError::new(
                        "path_norm requires a string argument".to_string(),
                    )),
                }
            }
            "path_is_abs" | "path_is_absolute" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "path_is_abs requires 1 argument".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => Ok(Value::Boolean(Path::new(path.as_ref()).is_absolute())),
                    _ => Err(RuntimeError::new(
                        "path_is_abs requires a string argument".to_string(),
                    )),
                }
            }
            "path_sep" | "path_separator" => {
                Ok(Value::String(Arc::from(std::path::MAIN_SEPARATOR.to_string())))
            }
            _ => Err(RuntimeError::new(format!(
                "Unknown path function: {}",
                name
            ))),
        }
    }

    fn path_join(paths: &[String]) -> Result<Value, RuntimeError> {
        if paths.is_empty() {
            return Ok(Value::String(Arc::from(String::new())));
        }

        let mut result = PathBuf::from(&paths[0]);
        for path in paths.iter().skip(1) {
            result = result.join(path);
        }

        Ok(Value::String(Arc::from(result.to_string_lossy().to_string())))
    }

    fn path_dir(path: &str) -> Result<Value, RuntimeError> {
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            Ok(Value::String(Arc::from(parent.to_string_lossy().to_string())))
        } else {
            Ok(Value::String(Arc::from(".".to_string())))
        }
    }

    fn path_base(path: &str) -> Result<Value, RuntimeError> {
        let p = Path::new(path);
        if let Some(file_name) = p.file_name() {
            Ok(Value::String(Arc::from(file_name.to_string_lossy().to_string())))
        } else {
            Ok(Value::String(Arc::from(String::new())))
        }
    }

    fn path_ext(path: &str) -> Result<Value, RuntimeError> {
        let p = Path::new(path);
        if let Some(ext) = p.extension() {
            Ok(Value::String(Arc::from(format!(".{}", ext.to_string_lossy()))))
        } else {
            Ok(Value::String(Arc::from(String::new())))
        }
    }

    fn path_stem(path: &str) -> Result<Value, RuntimeError> {
        let p = Path::new(path);
        if let Some(stem) = p.file_stem() {
            Ok(Value::String(Arc::from(stem.to_string_lossy().to_string())))
        } else {
            Ok(Value::String(Arc::from(String::new())))
        }
    }

    fn path_abs(path: &str) -> Result<Value, RuntimeError> {
        let p = Path::new(path);
        match p.canonicalize() {
            Ok(abs_path) => Ok(Value::String(Arc::from(abs_path.to_string_lossy().to_string()))),
            Err(_) => {
                // If canonicalize fails, try to make it absolute relative to current dir
                let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let abs_path = current_dir.join(p);
                Ok(Value::String(Arc::from(abs_path.to_string_lossy().to_string())))
            }
        }
    }

    fn path_norm(path: &str) -> Result<Value, RuntimeError> {
        let p = Path::new(path);
        let components: Vec<String> = p
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                std::path::Component::RootDir => Some("/".to_string()),
                std::path::Component::CurDir => Some(".".to_string()),
                std::path::Component::ParentDir => Some("..".to_string()),
                std::path::Component::Prefix(_) => None,
            })
            .collect();

        // Simple normalization: remove . and resolve ..
        let mut normalized = Vec::new();
        for component in components {
            if component == "." {
                continue;
            } else if component == ".." {
                normalized.pop();
            } else {
                normalized.push(component);
            }
        }

        let result = if normalized.is_empty() {
            ".".to_string()
        } else {
            normalized.join(std::path::MAIN_SEPARATOR_STR)
        };

        Ok(Value::String(Arc::from(result)))
    }
}
