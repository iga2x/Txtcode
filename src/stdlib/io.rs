use crate::runtime::{Value, RuntimeError};
use std::fs;
use std::path::PathBuf;
use std::env;

/// I/O library
pub struct IOLib;

impl IOLib {
    /// Validate and sanitize file paths to prevent path traversal attacks
    fn validate_path(path: &str) -> Result<PathBuf, RuntimeError> {
        // Check length
        if path.len() > 4096 {
            return Err(RuntimeError::new("Path too long (max 4096 chars)".to_string())
                .with_hint("File paths must be 4096 characters or less.".to_string()));
        }
        
        // Check for null bytes
        if path.contains('\0') {
            return Err(RuntimeError::new("Null bytes not allowed in paths".to_string()));
        }
        
        // Check for path traversal sequences (but allow absolute paths starting with /)
        if path.contains("..") {
            return Err(RuntimeError::new("Path traversal not allowed (cannot use '..')".to_string())
                .with_hint("Paths cannot contain '..' sequences for security reasons. Use absolute paths or paths relative to the current directory.".to_string()));
        }
        
        // Validate UTF-8 (PathBuf will handle this, but check explicitly)
        if !path.is_char_boundary(0) {
            return Err(RuntimeError::new("Invalid UTF-8 in path".to_string()));
        }
        
        let path_buf = PathBuf::from(path);
        
        // If path is already absolute (starts with / on Unix or C:\ on Windows), validate it directly
        if path_buf.is_absolute() {
            // On Unix, allow absolute paths starting with /tmp, /var/tmp, or current working directory
            // On Windows, allow absolute paths
            #[cfg(unix)]
            {
                // Check if it's an allowed absolute path (e.g., /tmp, /var/tmp)
                let path_str = path_buf.to_string_lossy();
                if path_str.starts_with("/tmp/") || path_str == "/tmp" ||
                   path_str.starts_with("/var/tmp/") || path_str == "/var/tmp" {
                    // Try to canonicalize
                    match path_buf.canonicalize() {
                        Ok(canonical) => return Ok(canonical),
                        Err(_) => {
                            // Path doesn't exist yet, but parent should
                            if let Some(parent) = path_buf.parent() {
                                match parent.canonicalize() {
                                    Ok(_) => return Ok(path_buf),
                                    Err(_) => {
                                        return Err(RuntimeError::new(format!("Parent directory does not exist: {}", parent.display()))
                                            .with_hint("The parent directory of the path must exist.".to_string()));
                                    }
                                }
                            }
                            return Ok(path_buf);
                        }
                    }
                }
            }
            
            // For other absolute paths, check if they're within current working directory
            // or allow if permission system has explicitly granted access (handled by permission checker)
            let current_dir = env::current_dir()
                .unwrap_or_else(|_| env::temp_dir());
            
            // Try to canonicalize
            match path_buf.canonicalize() {
                Ok(canonical) => {
                    // Check if canonicalized path is within current directory
                    if canonical.starts_with(&current_dir) {
                        return Ok(canonical);
                    }
                    // If not, but it's /tmp or /var/tmp, allow it
                    #[cfg(unix)]
                    {
                        let canonical_str = canonical.to_string_lossy();
                        if canonical_str.starts_with("/tmp/") || canonical_str == "/tmp" ||
                           canonical_str.starts_with("/var/tmp/") || canonical_str == "/var/tmp" {
                            return Ok(canonical);
                        }
                    }
                    // Otherwise, return error - permission system should grant access if needed
                    return Err(RuntimeError::new(format!("Path outside allowed directory: {}", canonical.display()))
                        .with_hint("Paths must be within the current working directory or explicitly allowed via permissions (e.g., /tmp/*)".to_string()));
                }
                Err(_) => {
                    // Path doesn't exist yet - check if parent is within allowed directory
                    if let Some(parent) = path_buf.parent() {
                        if parent.is_absolute() {
                            #[cfg(unix)]
                            {
                                let parent_str = parent.to_string_lossy();
                                if parent_str.starts_with("/tmp/") || parent_str == "/tmp" ||
                                   parent_str.starts_with("/var/tmp/") || parent_str == "/var/tmp" {
                                    return Ok(path_buf);
                                }
                            }
                        }
                        match parent.canonicalize() {
                            Ok(canonical_parent) => {
                                if canonical_parent.starts_with(&current_dir) {
                                    return Ok(path_buf);
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    // Fall through to relative path handling
                }
            }
        }
        
        // For relative paths, ensure they're within current working directory
        let base = env::current_dir()
            .unwrap_or_else(|_| env::temp_dir());
        
        // Join and canonicalize path
        let full_path = base.join(path);
        
        // Try to canonicalize (resolves symlinks and . components)
        match full_path.canonicalize() {
            Ok(canonical) => {
                // Ensure canonicalized path is within base directory
                if !canonical.starts_with(&base) {
                    return Err(RuntimeError::new("Path outside allowed directory".to_string())
                        .with_hint("Paths must be within the current working directory for security reasons.".to_string()));
                }
                Ok(canonical)
            }
            Err(_) => {
                // If canonicalization fails (path doesn't exist yet), use the joined path
                // but still check it starts with base
                if !full_path.starts_with(&base) {
                    return Err(RuntimeError::new("Path outside allowed directory".to_string())
                        .with_hint("Paths must be within the current working directory for security reasons.".to_string()));
                }
                Ok(full_path)
            }
        }
    }
}

impl IOLib {
    /// Call an I/O library function
    /// permission_checker: optional permission checker for permission enforcement
    pub fn call_function(name: &str, args: &[Value], permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>) -> Result<Value, RuntimeError> {
        match name {
            "read_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("read_file requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_str())
                            )?;
                        }
                        
                        let validated_path = Self::validate_path(path)?;
                        fs::read_to_string(&validated_path)
                            .map(Value::String)
                            .map_err(|e| RuntimeError::new(format!("Failed to read file: {}", e)))
                    }
                    _ => Err(RuntimeError::new("read_file requires a string path".to_string())),
                }
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("write_file requires 2 arguments (path, content)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(path.as_str())
                            )?;
                        }
                        
                        let validated_path = Self::validate_path(path)?;
                        fs::write(&validated_path, content)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to write file: {}", e)))
                    }
                    _ => Err(RuntimeError::new("write_file requires strings".to_string())),
                }
            }
            "file_exists" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("file_exists requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => Ok(Value::Boolean(validated_path.exists())),
                            Err(_) => Ok(Value::Boolean(false)), // Invalid path = doesn't exist
                        }
                    }
                    _ => Err(RuntimeError::new("file_exists requires a string path".to_string())),
                }
            }
            "list_dir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("list_dir requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        let entries: Result<Vec<Value>, RuntimeError> = fs::read_dir(&validated_path)
                            .map_err(|e| RuntimeError::new(format!("Failed to read directory: {}", e)))?
                            .map(|entry: Result<std::fs::DirEntry, std::io::Error>| {
                                entry
                                    .map(|e: std::fs::DirEntry| Value::String(e.path().to_string_lossy().to_string()))
                                    .map_err(|e| RuntimeError::new(format!("Failed to read entry: {}", e)))
                            })
                            .collect();
                        entries.map(Value::Array)
                    }
                    _ => Err(RuntimeError::new("list_dir requires a string path".to_string())),
                }
            }
            "is_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("is_file requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => Ok(Value::Boolean(validated_path.is_file())),
                            Err(_) => Ok(Value::Boolean(false)), // Invalid path = not a file
                        }
                    }
                    _ => Err(RuntimeError::new("is_file requires a string path".to_string())),
                }
            }
            "is_dir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("is_dir requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => Ok(Value::Boolean(validated_path.is_dir())),
                            Err(_) => Ok(Value::Boolean(false)), // Invalid path = not a dir
                        }
                    }
                    _ => Err(RuntimeError::new("is_dir requires a string path".to_string())),
                }
            }
            "mkdir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("mkdir requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        fs::create_dir_all(&validated_path)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to create directory: {}", e)))
                    }
                    _ => Err(RuntimeError::new("mkdir requires a string path".to_string())),
                }
            }
            "rmdir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("rmdir requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        fs::remove_dir(&validated_path)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to remove directory: {}", e)))
                    }
                    _ => Err(RuntimeError::new("rmdir requires a string path".to_string())),
                }
            }
            "delete" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("delete requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("delete".to_string()),
                                Some(path.as_str())
                            )?;
                        }
                        
                        let validated_path = Self::validate_path(path)?;
                        if validated_path.is_dir() {
                            fs::remove_dir_all(&validated_path)
                                .map(|_| Value::Null)
                                .map_err(|e| RuntimeError::new(format!("Failed to delete directory: {}", e)))
                        } else {
                            fs::remove_file(&validated_path)
                                .map(|_| Value::Null)
                                .map_err(|e| RuntimeError::new(format!("Failed to delete file: {}", e)))
                        }
                    }
                    _ => Err(RuntimeError::new("delete requires a string path".to_string())),
                }
            }
            "read_file_binary" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("read_file_binary requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        let data = fs::read(&validated_path)
                            .map_err(|e| RuntimeError::new(format!("Failed to read file: {}", e)))?;
                        // Return as hex string for binary data
                        Ok(Value::String(hex::encode(data)))
                    }
                    _ => Err(RuntimeError::new("read_file_binary requires a string path".to_string())),
                }
            }
            "write_file_binary" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("write_file_binary requires 2 arguments (path, data)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(data_hex)) => {
                        let validated_path = Self::validate_path(path)?;
                        let data = hex::decode(data_hex)
                            .map_err(|e| RuntimeError::new(format!("Invalid hex data: {}", e)))?;
                        fs::write(&validated_path, data)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to write file: {}", e)))
                    }
                    _ => Err(RuntimeError::new("write_file_binary requires string path and hex data".to_string())),
                }
            }
            "append_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("append_file requires 2 arguments (path, content)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        let validated_path = Self::validate_path(path)?;
                        use std::io::Write;
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&validated_path)
                            .map_err(|e| RuntimeError::new(format!("Failed to open file: {}", e)))?;
                        file.write_all(content.as_bytes())
                            .map_err(|e| RuntimeError::new(format!("Failed to write to file: {}", e)))?;
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError::new("append_file requires strings".to_string())),
                }
            }
            "copy_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("copy_file requires 2 arguments (src, dst)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(src), Value::String(dst)) => {
                        let validated_src = Self::validate_path(src)?;
                        let validated_dst = Self::validate_path(dst)?;
                        fs::copy(&validated_src, &validated_dst)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to copy file: {}", e)))
                    }
                    _ => Err(RuntimeError::new("copy_file requires string paths".to_string())),
                }
            }
            "move_file" | "rename_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("move_file requires 2 arguments (src, dst)".to_string()));
                }
                match (&args[0], &args[1]) {
                    (Value::String(src), Value::String(dst)) => {
                        let validated_src = Self::validate_path(src)?;
                        let validated_dst = Self::validate_path(dst)?;
                        fs::rename(&validated_src, &validated_dst)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to move file: {}", e)))
                    }
                    _ => Err(RuntimeError::new("move_file requires string paths".to_string())),
                }
            }
            "file_size" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("file_size requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => {
                                match fs::metadata(&validated_path) {
                                    Ok(meta) => Ok(Value::Integer(meta.len() as i64)),
                                    Err(e) => Err(RuntimeError::new(format!("Failed to get file size: {}", e))),
                                }
                            }
                            Err(_) => Ok(Value::Integer(0)),
                        }
                    }
                    _ => Err(RuntimeError::new("file_size requires a string path".to_string())),
                }
            }
            "file_modified" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("file_modified requires 1 argument (path)".to_string()));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => {
                                match fs::metadata(&validated_path) {
                                    Ok(meta) => {
                                        if let Ok(modified) = meta.modified() {
                                            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                                                Ok(Value::Integer(duration.as_secs() as i64))
                                            } else {
                                                Ok(Value::Integer(0))
                                            }
                                        } else {
                                            Ok(Value::Integer(0))
                                        }
                                    }
                                    Err(e) => Err(RuntimeError::new(format!("Failed to get file metadata: {}", e))),
                                }
                            }
                            Err(_) => Ok(Value::Integer(0)),
                        }
                    }
                    _ => Err(RuntimeError::new("file_modified requires a string path".to_string())),
                }
            }
            _ => Err(RuntimeError::new(format!("Unknown I/O function: {}", name))),
        }
    }
}
