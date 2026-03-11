use crate::runtime::{RuntimeError, Value};
use std::env;
use std::fs;
use std::path::PathBuf;

/// I/O library
pub struct IOLib;

impl IOLib {
    /// Validate and sanitize file paths to prevent path traversal attacks
    fn validate_path(path: &str) -> Result<PathBuf, RuntimeError> {
        // Check length
        if path.len() > 4096 {
            return Err(
                RuntimeError::new("Path too long (max 4096 chars)".to_string())
                    .with_hint("File paths must be 4096 characters or less.".to_string()),
            );
        }

        // Check for null bytes
        if path.contains('\0') {
            return Err(RuntimeError::new(
                "Null bytes not allowed in paths".to_string(),
            ));
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
                if path_str.starts_with("/tmp/")
                    || path_str == "/tmp"
                    || path_str.starts_with("/var/tmp/")
                    || path_str == "/var/tmp"
                {
                    // Try to canonicalize
                    match path_buf.canonicalize() {
                        Ok(canonical) => return Ok(canonical),
                        Err(_) => {
                            // Path doesn't exist yet, but parent should
                            if let Some(parent) = path_buf.parent() {
                                match parent.canonicalize() {
                                    Ok(_) => return Ok(path_buf),
                                    Err(_) => {
                                        return Err(RuntimeError::new(format!(
                                            "Parent directory does not exist: {}",
                                            parent.display()
                                        ))
                                        .with_hint(
                                            "The parent directory of the path must exist."
                                                .to_string(),
                                        ));
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
            let current_dir = env::current_dir().unwrap_or_else(|_| env::temp_dir());

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
                        if canonical_str.starts_with("/tmp/")
                            || canonical_str == "/tmp"
                            || canonical_str.starts_with("/var/tmp/")
                            || canonical_str == "/var/tmp"
                        {
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
                                if parent_str.starts_with("/tmp/")
                                    || parent_str == "/tmp"
                                    || parent_str.starts_with("/var/tmp/")
                                    || parent_str == "/var/tmp"
                                {
                                    return Ok(path_buf);
                                }
                            }
                        }
                        if let Ok(canonical_parent) = parent.canonicalize() {
                            if canonical_parent.starts_with(&current_dir) {
                                return Ok(path_buf);
                            }
                        }
                    }
                    // Fall through to relative path handling
                }
            }
        }

        // For relative paths, ensure they're within current working directory
        let base = env::current_dir().unwrap_or_else(|_| env::temp_dir());

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
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        match name {
            "read_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "read_file requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_str()),
                            )?;
                        }

                        let validated_path = Self::validate_path(path)?;
                        fs::read_to_string(&validated_path)
                            .map(Value::String)
                            .map_err(|e| RuntimeError::new(format!("Failed to read file: {}", e)))
                    }
                    _ => Err(RuntimeError::new(
                        "read_file requires a string path".to_string(),
                    )),
                }
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "write_file requires 2 arguments (path, content)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(path.as_str()),
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
                    return Err(RuntimeError::new(
                        "file_exists requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => Ok(Value::Boolean(validated_path.exists())),
                            Err(_) => Ok(Value::Boolean(false)), // Invalid path = doesn't exist
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "file_exists requires a string path".to_string(),
                    )),
                }
            }
            "list_dir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "list_dir requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        let entries: Result<Vec<Value>, RuntimeError> =
                            fs::read_dir(&validated_path)
                                .map_err(|e| {
                                    RuntimeError::new(format!("Failed to read directory: {}", e))
                                })?
                                .map(|entry: Result<std::fs::DirEntry, std::io::Error>| {
                                    entry
                                        .map(|e: std::fs::DirEntry| {
                                            Value::String(e.path().to_string_lossy().to_string())
                                        })
                                        .map_err(|e| {
                                            RuntimeError::new(format!(
                                                "Failed to read entry: {}",
                                                e
                                            ))
                                        })
                                })
                                .collect();
                        entries.map(Value::Array)
                    }
                    _ => Err(RuntimeError::new(
                        "list_dir requires a string path".to_string(),
                    )),
                }
            }
            "is_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "is_file requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => Ok(Value::Boolean(validated_path.is_file())),
                            Err(_) => Ok(Value::Boolean(false)), // Invalid path = not a file
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "is_file requires a string path".to_string(),
                    )),
                }
            }
            "is_dir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "is_dir requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        match Self::validate_path(path) {
                            Ok(validated_path) => Ok(Value::Boolean(validated_path.is_dir())),
                            Err(_) => Ok(Value::Boolean(false)), // Invalid path = not a dir
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "is_dir requires a string path".to_string(),
                    )),
                }
            }
            "mkdir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "mkdir requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        fs::create_dir_all(&validated_path)
                            .map(|_| Value::Null)
                            .map_err(|e| {
                                RuntimeError::new(format!("Failed to create directory: {}", e))
                            })
                    }
                    _ => Err(RuntimeError::new(
                        "mkdir requires a string path".to_string(),
                    )),
                }
            }
            "rmdir" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "rmdir requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        fs::remove_dir(&validated_path)
                            .map(|_| Value::Null)
                            .map_err(|e| {
                                RuntimeError::new(format!("Failed to remove directory: {}", e))
                            })
                    }
                    _ => Err(RuntimeError::new(
                        "rmdir requires a string path".to_string(),
                    )),
                }
            }
            "delete" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "delete requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        // Check permission if checker is available
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("delete".to_string()),
                                Some(path.as_str()),
                            )?;
                        }

                        let validated_path = Self::validate_path(path)?;
                        if validated_path.is_dir() {
                            fs::remove_dir_all(&validated_path)
                                .map(|_| Value::Null)
                                .map_err(|e| {
                                    RuntimeError::new(format!("Failed to delete directory: {}", e))
                                })
                        } else {
                            fs::remove_file(&validated_path)
                                .map(|_| Value::Null)
                                .map_err(|e| {
                                    RuntimeError::new(format!("Failed to delete file: {}", e))
                                })
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "delete requires a string path".to_string(),
                    )),
                }
            }
            "read_file_binary" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "read_file_binary requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        let validated_path = Self::validate_path(path)?;
                        let data = fs::read(&validated_path).map_err(|e| {
                            RuntimeError::new(format!("Failed to read file: {}", e))
                        })?;
                        // Return as hex string for binary data
                        Ok(Value::String(hex::encode(data)))
                    }
                    _ => Err(RuntimeError::new(
                        "read_file_binary requires a string path".to_string(),
                    )),
                }
            }
            "write_file_binary" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "write_file_binary requires 2 arguments (path, data)".to_string(),
                    ));
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
                    _ => Err(RuntimeError::new(
                        "write_file_binary requires string path and hex data".to_string(),
                    )),
                }
            }
            "append_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "append_file requires 2 arguments (path, content)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        let validated_path = Self::validate_path(path)?;
                        use std::io::Write;
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&validated_path)
                            .map_err(|e| {
                                RuntimeError::new(format!("Failed to open file: {}", e))
                            })?;
                        file.write_all(content.as_bytes()).map_err(|e| {
                            RuntimeError::new(format!("Failed to write to file: {}", e))
                        })?;
                        Ok(Value::Null)
                    }
                    _ => Err(RuntimeError::new(
                        "append_file requires strings".to_string(),
                    )),
                }
            }
            "copy_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "copy_file requires 2 arguments (src, dst)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(src), Value::String(dst)) => {
                        let validated_src = Self::validate_path(src)?;
                        let validated_dst = Self::validate_path(dst)?;
                        fs::copy(&validated_src, &validated_dst)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to copy file: {}", e)))
                    }
                    _ => Err(RuntimeError::new(
                        "copy_file requires string paths".to_string(),
                    )),
                }
            }
            "move_file" | "rename_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "move_file requires 2 arguments (src, dst)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(src), Value::String(dst)) => {
                        let validated_src = Self::validate_path(src)?;
                        let validated_dst = Self::validate_path(dst)?;
                        fs::rename(&validated_src, &validated_dst)
                            .map(|_| Value::Null)
                            .map_err(|e| RuntimeError::new(format!("Failed to move file: {}", e)))
                    }
                    _ => Err(RuntimeError::new(
                        "move_file requires string paths".to_string(),
                    )),
                }
            }
            "file_size" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "file_size requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => match Self::validate_path(path) {
                        Ok(validated_path) => match fs::metadata(&validated_path) {
                            Ok(meta) => Ok(Value::Integer(meta.len() as i64)),
                            Err(e) => {
                                Err(RuntimeError::new(format!("Failed to get file size: {}", e)))
                            }
                        },
                        Err(_) => Ok(Value::Integer(0)),
                    },
                    _ => Err(RuntimeError::new(
                        "file_size requires a string path".to_string(),
                    )),
                }
            }
            "file_modified" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "file_modified requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => match Self::validate_path(path) {
                        Ok(validated_path) => match fs::metadata(&validated_path) {
                            Ok(meta) => {
                                if let Ok(modified) = meta.modified() {
                                    if let Ok(duration) =
                                        modified.duration_since(std::time::UNIX_EPOCH)
                                    {
                                        Ok(Value::Integer(duration.as_secs() as i64))
                                    } else {
                                        Ok(Value::Integer(0))
                                    }
                                } else {
                                    Ok(Value::Integer(0))
                                }
                            }
                            Err(e) => Err(RuntimeError::new(format!(
                                "Failed to get file metadata: {}",
                                e
                            ))),
                        },
                        Err(_) => Ok(Value::Integer(0)),
                    },
                    _ => Err(RuntimeError::new(
                        "file_modified requires a string path".to_string(),
                    )),
                }
            }
            "read_lines" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "read_lines requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_str()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let content = fs::read_to_string(&validated_path).map_err(|e| {
                            RuntimeError::new(format!("Failed to read file: {}", e))
                        })?;
                        let lines: Vec<Value> = content
                            .lines()
                            .map(|l| Value::String(l.to_string()))
                            .collect();
                        Ok(Value::Array(lines))
                    }
                    _ => Err(RuntimeError::new(
                        "read_lines requires a string path".to_string(),
                    )),
                }
            }
            "read_csv" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "read_csv requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_str()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let content = fs::read_to_string(&validated_path).map_err(|e| {
                            RuntimeError::new(format!("Failed to read CSV file: {}", e))
                        })?;
                        let mut rows: Vec<Value> = Vec::new();
                        for line in content.lines() {
                            if line.trim().is_empty() {
                                continue;
                            }
                            let mut fields: Vec<Value> = Vec::new();
                            let mut field = String::new();
                            let mut in_quotes = false;
                            let mut chars = line.chars().peekable();
                            while let Some(c) = chars.next() {
                                match c {
                                    '"' if !in_quotes => {
                                        in_quotes = true;
                                    }
                                    '"' if in_quotes => {
                                        if chars.peek() == Some(&'"') {
                                            chars.next();
                                            field.push('"');
                                        } else {
                                            in_quotes = false;
                                        }
                                    }
                                    ',' if !in_quotes => {
                                        fields.push(Value::String(field.clone()));
                                        field.clear();
                                    }
                                    _ => {
                                        field.push(c);
                                    }
                                }
                            }
                            fields.push(Value::String(field));
                            rows.push(Value::Array(fields));
                        }
                        Ok(Value::Array(rows))
                    }
                    _ => Err(RuntimeError::new(
                        "read_csv requires a string path".to_string(),
                    )),
                }
            }
            "temp_file" => {
                if !args.is_empty() {
                    return Err(RuntimeError::new(
                        "temp_file takes no arguments".to_string(),
                    ));
                }
                let tmp_dir = env::temp_dir();
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let pid = std::process::id();
                let filename = format!("txtcode_tmp_{}_{}", pid, nanos);
                let tmp_path = tmp_dir.join(&filename);
                fs::File::create(&tmp_path)
                    .map_err(|e| RuntimeError::new(format!("Failed to create temp file: {}", e)))?;
                Ok(Value::String(tmp_path.to_string_lossy().to_string()))
            }
            "watch_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "watch_file requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        let mut map = std::collections::HashMap::new();
                        map.insert("path".to_string(), Value::String(path.clone()));
                        match Self::validate_path(path) {
                            Ok(validated_path) => {
                                let exists = validated_path.exists();
                                map.insert("exists".to_string(), Value::Boolean(exists));
                                if exists {
                                    if let Ok(meta) = fs::metadata(&validated_path) {
                                        map.insert(
                                            "size".to_string(),
                                            Value::Integer(meta.len() as i64),
                                        );
                                        if let Ok(modified) = meta.modified() {
                                            if let Ok(dur) =
                                                modified.duration_since(std::time::UNIX_EPOCH)
                                            {
                                                map.insert(
                                                    "modified".to_string(),
                                                    Value::Integer(dur.as_secs() as i64),
                                                );
                                            } else {
                                                map.insert(
                                                    "modified".to_string(),
                                                    Value::Integer(0),
                                                );
                                            }
                                        } else {
                                            map.insert("modified".to_string(), Value::Integer(0));
                                        }
                                    }
                                } else {
                                    map.insert("size".to_string(), Value::Integer(0));
                                    map.insert("modified".to_string(), Value::Integer(0));
                                }
                            }
                            Err(_) => {
                                map.insert("exists".to_string(), Value::Boolean(false));
                                map.insert("size".to_string(), Value::Integer(0));
                                map.insert("modified".to_string(), Value::Integer(0));
                            }
                        }
                        Ok(Value::Map(map))
                    }
                    _ => Err(RuntimeError::new(
                        "watch_file requires a string path".to_string(),
                    )),
                }
            }
            "symlink_create" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "symlink_create requires 2 arguments (target, link_path)".to_string(),
                    ));
                }
                match (&args[0], &args[1]) {
                    (Value::String(target), Value::String(link_path)) => {
                        let validated_link = Self::validate_path(link_path)?;
                        #[cfg(unix)]
                        {
                            std::os::unix::fs::symlink(target, &validated_link)
                                .map(|_| Value::Null)
                                .map_err(|e| {
                                    RuntimeError::new(format!("Failed to create symlink: {}", e))
                                })
                        }
                        #[cfg(not(unix))]
                        {
                            let _ = (target, validated_link);
                            Err(RuntimeError::new(
                                "symlink_create is not supported on Windows".to_string(),
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        "symlink_create requires string arguments".to_string(),
                    )),
                }
            }
            "zip_create" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "zip_create requires at least 1 argument: output_path".to_string(),
                    ));
                }
                let output_path = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "zip_create: output_path must be a string".to_string(),
                        ))
                    }
                };
                let file = std::fs::File::create(&output_path).map_err(|e| {
                    RuntimeError::new(format!("zip_create: cannot create {}: {}", output_path, e))
                })?;
                let mut zip_writer = zip::ZipWriter::new(file);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                let file_args: Vec<String> = if args.len() == 2 {
                    if let Value::Array(arr) = &args[1] {
                        arr.iter()
                            .filter_map(|v| {
                                if let Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else if let Value::String(s) = &args[1] {
                        vec![s.clone()]
                    } else {
                        return Err(RuntimeError::new(
                            "zip_create: second argument must be a string or array of strings"
                                .to_string(),
                        ));
                    }
                } else {
                    args[1..]
                        .iter()
                        .filter_map(|v| {
                            if let Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                };
                for src_path in &file_args {
                    let name = std::path::Path::new(src_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(src_path.as_str());
                    zip_writer.start_file(name, options).map_err(|e| {
                        RuntimeError::new(format!("zip_create: error adding {}: {}", src_path, e))
                    })?;
                    let data = std::fs::read(src_path).map_err(|e| {
                        RuntimeError::new(format!("zip_create: cannot read {}: {}", src_path, e))
                    })?;
                    use std::io::Write;
                    zip_writer.write_all(&data).map_err(|e| {
                        RuntimeError::new(format!("zip_create: write error: {}", e))
                    })?;
                }
                zip_writer
                    .finish()
                    .map_err(|e| RuntimeError::new(format!("zip_create: finalize error: {}", e)))?;
                Ok(Value::String(output_path))
            }
            "zip_extract" => {
                if args.len() < 2 {
                    return Err(RuntimeError::new(
                        "zip_extract requires 2 arguments: archive_path, output_dir".to_string(),
                    ));
                }
                let archive_path = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "zip_extract: archive_path must be a string".to_string(),
                        ))
                    }
                };
                let output_dir = match &args[1] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            "zip_extract: output_dir must be a string".to_string(),
                        ))
                    }
                };
                let file = std::fs::File::open(&archive_path).map_err(|e| {
                    RuntimeError::new(format!("zip_extract: cannot open {}: {}", archive_path, e))
                })?;
                let mut archive = zip::ZipArchive::new(file).map_err(|e| {
                    RuntimeError::new(format!("zip_extract: invalid archive: {}", e))
                })?;
                std::fs::create_dir_all(&output_dir).map_err(|e| {
                    RuntimeError::new(format!("zip_extract: cannot create output dir: {}", e))
                })?;
                let count = archive.len();
                for i in 0..count {
                    let mut entry = archive.by_index(i).map_err(|e| {
                        RuntimeError::new(format!("zip_extract: error reading entry {}: {}", i, e))
                    })?;
                    let out_path = std::path::Path::new(&output_dir).join(entry.name());
                    if entry.is_dir() {
                        std::fs::create_dir_all(&out_path).map_err(|e| {
                            RuntimeError::new(format!("zip_extract: mkdir error: {}", e))
                        })?;
                    } else {
                        if let Some(parent) = out_path.parent() {
                            std::fs::create_dir_all(parent).map_err(|e| {
                                RuntimeError::new(format!("zip_extract: mkdir error: {}", e))
                            })?;
                        }
                        let mut out_file = std::fs::File::create(&out_path).map_err(|e| {
                            RuntimeError::new(format!("zip_extract: create error: {}", e))
                        })?;
                        std::io::copy(&mut entry, &mut out_file).map_err(|e| {
                            RuntimeError::new(format!("zip_extract: copy error: {}", e))
                        })?;
                    }
                }
                Ok(Value::Integer(count as i64))
            }
            _ => Err(RuntimeError::new(format!("Unknown I/O function: {}", name))),
        }
    }
}
