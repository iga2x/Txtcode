use crate::runtime::{RuntimeError, Value};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

// ── File handle registry ─────────────────────────────────────────────────────
// Stores open file handles keyed by integer ID.
// Handles are returned as Value::Integer to user code.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;

lazy_static::lazy_static! {
    static ref READ_HANDLES: Mutex<HashMap<i64, BufReader<fs::File>>> = Mutex::new(HashMap::new());
    static ref WRITE_HANDLES: Mutex<HashMap<i64, std::io::BufWriter<fs::File>>> = Mutex::new(HashMap::new());
    static ref NEXT_HANDLE_ID: Mutex<i64> = Mutex::new(1);
    // Group 27.4: CSV streaming handles
    static ref CSV_READ_HANDLES: Mutex<HashMap<i64, (BufReader<fs::File>, char)>> =
        Mutex::new(HashMap::new());
    static ref CSV_WRITE_HANDLES: Mutex<HashMap<i64, (std::io::BufWriter<fs::File>, char)>> =
        Mutex::new(HashMap::new());
}

fn next_handle_id() -> i64 {
    let mut id = NEXT_HANDLE_ID.lock().unwrap();
    let result = *id;
    *id += 1;
    result
}

/// I/O library
pub struct IOLib;

impl IOLib {
    /// Validate a zip entry name against an output directory to prevent zip-slip attacks.
    ///
    /// Anchors to `output_dir` (not `current_dir()`), which must already exist.
    /// Rejects entries with `..` path components and entries that resolve outside `output_dir`.
    fn validate_zip_entry_path(
        output_dir: &std::path::Path,
        entry_name: &str,
    ) -> Result<PathBuf, RuntimeError> {
        // Reject any entry component that is ParentDir (..)
        let entry_path = std::path::Path::new(entry_name);
        for component in entry_path.components() {
            if component == std::path::Component::ParentDir {
                return Err(RuntimeError::new(format!(
                    "zip_extract: entry '{}' contains '..' path traversal",
                    entry_name
                )));
            }
        }

        // Canonicalize the output directory (it must already exist at this point)
        let canonical_output = output_dir.canonicalize().map_err(|e| {
            RuntimeError::new(format!(
                "zip_extract: cannot canonicalize output dir '{}': {}",
                output_dir.display(),
                e
            ))
        })?;

        // Construct the joined path and verify it stays inside canonical_output
        let out_path = canonical_output.join(entry_name);
        if !out_path.starts_with(&canonical_output) {
            return Err(RuntimeError::new(format!(
                "zip_extract: entry '{}' would escape the output directory",
                entry_name
            )));
        }

        Ok(out_path)
    }

    /// Public wrapper for `validate_path` used by external callers such as the REPL.
    pub fn validate_path_pub(path: &str) -> Result<PathBuf, RuntimeError> {
        Self::validate_path(path)
    }

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
    /// Call an I/O library function.
    ///
    /// `permission_checker`: Must be `Some(checker)` in all VM-dispatched calls.
    /// Pass `None` only in trusted internal Rust contexts (unit tests, tool executors
    /// that perform their own permission checks upstream).
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(debug_assertions)]
        if permission_checker.is_none() {
            crate::tools::logger::log_warn(&format!(
                "stdlib internal: '{}' called without permission_checker — trusted path only", name
            ));
        }
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
                                Some(path.as_ref()),
                            )?;
                        }

                        let validated_path = Self::validate_path(path)?;
                        match fs::read_to_string(&validated_path) {
                            Ok(content) => Ok(Value::String(Arc::from(content))),
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                Ok(crate::stdlib::errors::file_not_found(path))
                            }
                            Err(e) => Err(RuntimeError::new(format!("Failed to read file: {}", e))),
                        }
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
                                Some(path.as_ref()),
                            )?;
                        }

                        let validated_path = Self::validate_path(path)?;
                        fs::write(&validated_path, content.as_bytes())
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let entries: Result<Vec<Value>, RuntimeError> =
                            fs::read_dir(&validated_path)
                                .map_err(|e| {
                                    RuntimeError::new(format!("Failed to read directory: {}", e))
                                })?
                                .map(|entry: Result<std::fs::DirEntry, std::io::Error>| {
                                    entry
                                        .map(|e: std::fs::DirEntry| {
                                            Value::String(Arc::from(e.path().to_string_lossy().to_string()))
                                        })
                                        .map_err(|e| {
                                            RuntimeError::new(format!(
                                                "Failed to read entry: {}",
                                                e
                                            ))
                                        })
                                })
                                .collect();
                        entries.map(|mut v| {
                            v.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
                            Value::Array(v)
                        })
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("delete".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
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
                                Some(path.as_ref()),
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let data = fs::read(&validated_path).map_err(|e| {
                            RuntimeError::new(format!("Failed to read file: {}", e))
                        })?;
                        // Return as hex string for binary data
                        Ok(Value::String(Arc::from(hex::encode(data))))
                    }
                    _ => Err(RuntimeError::new(
                        "read_file_binary requires a string path".to_string(),
                    )),
                }
            }
            // read_file_bytes(path) → Value::Bytes (Task 2.2)
            "read_file_bytes" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "read_file_bytes requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let data = fs::read(&validated_path).map_err(|e| {
                            RuntimeError::new(format!("Failed to read file: {}", e))
                        })?;
                        Ok(Value::Bytes(data))
                    }
                    _ => Err(RuntimeError::new(
                        "read_file_bytes requires a string path".to_string(),
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let data = hex::decode(data_hex.as_ref())
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(path.as_ref()),
                            )?;
                        }
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("read".to_string()),
                                Some(src.as_ref()),
                            )?;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(dst.as_ref()),
                            )?;
                        }
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
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(src.as_ref()),
                            )?;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(dst.as_ref()),
                            )?;
                        }
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
                                Some(path.as_ref()),
                            )?;
                        }
                        let validated_path = Self::validate_path(path)?;
                        let content = fs::read_to_string(&validated_path).map_err(|e| {
                            RuntimeError::new(format!("Failed to read file: {}", e))
                        })?;
                        let lines: Vec<Value> = content
                            .lines()
                            .map(|l| Value::String(Arc::from(l.to_string())))
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
                                Some(path.as_ref()),
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
                                        fields.push(Value::String(Arc::from(field.clone())));
                                        field.clear();
                                    }
                                    _ => {
                                        field.push(c);
                                    }
                                }
                            }
                            fields.push(Value::String(Arc::from(field)));
                            rows.push(Value::Array(fields));
                        }
                        Ok(Value::Array(rows))
                    }
                    _ => Err(RuntimeError::new(
                        "read_csv requires a string path".to_string(),
                    )),
                }
            }
            // ── Streaming file I/O ───────────────────────────────────────────
            "file_open" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(RuntimeError::new("file_open requires 1 or 2 arguments (path, mode?)".to_string()));
                }
                let path = match &args[0] {
                    Value::String(p) => p.clone(),
                    _ => return Err(RuntimeError::new("file_open: path must be a string".to_string())),
                };
                let mode = if args.len() == 2 {
                    match &args[1] {
                        Value::String(m) => m.as_ref(),
                        _ => return Err(RuntimeError::new("file_open: mode must be a string".to_string())),
                    }
                } else {
                    "r"
                };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    let perm = if mode == "r" || mode == "read" { "read" } else { "write" };
                    checker.check_permission(&PermissionResource::FileSystem(perm.to_string()), None)?;
                }
                match mode {
                    "r" | "read" => {
                        let file = fs::File::open(path.as_ref())
                            .map_err(|e| RuntimeError::new(format!("file_open: cannot open '{}': {}", path, e)))?;
                        let id = next_handle_id();
                        READ_HANDLES.lock().unwrap().insert(id, BufReader::new(file));
                        Ok(Value::Integer(id))
                    }
                    "w" | "write" => {
                        let file = fs::File::create(path.as_ref())
                            .map_err(|e| RuntimeError::new(format!("file_open: cannot create '{}': {}", path, e)))?;
                        let id = next_handle_id();
                        WRITE_HANDLES.lock().unwrap().insert(id, std::io::BufWriter::new(file));
                        Ok(Value::Integer(id))
                    }
                    "a" | "append" => {
                        let file = fs::OpenOptions::new().append(true).create(true).open(path.as_ref())
                            .map_err(|e| RuntimeError::new(format!("file_open: cannot open '{}' for append: {}", path, e)))?;
                        let id = next_handle_id();
                        WRITE_HANDLES.lock().unwrap().insert(id, std::io::BufWriter::new(file));
                        Ok(Value::Integer(id))
                    }
                    other => Err(RuntimeError::new(format!("file_open: unknown mode '{}' (use r/w/a)", other))),
                }
            }
            "file_read_line" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("file_read_line requires 1 argument (handle)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("file_read_line: handle must be an integer".to_string())),
                };
                let mut handles = READ_HANDLES.lock().unwrap();
                let reader = handles.get_mut(&id)
                    .ok_or_else(|| RuntimeError::new(format!("file_read_line: invalid or closed handle {}", id)))?;
                let mut line = String::new();
                let bytes = reader.read_line(&mut line)
                    .map_err(|e| RuntimeError::new(format!("file_read_line: read error: {}", e)))?;
                if bytes == 0 {
                    Ok(Value::Null) // EOF
                } else {
                    // Strip trailing newline
                    if line.ends_with('\n') { line.pop(); }
                    if line.ends_with('\r') { line.pop(); }
                    Ok(Value::String(Arc::from(line)))
                }
            }
            "file_write_line" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new("file_write_line requires 2 arguments (handle, line)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("file_write_line: handle must be an integer".to_string())),
                };
                let line: String = match &args[1] {
                    Value::String(s) => s.to_string(),
                    other => other.to_string(),
                };
                let mut handles = WRITE_HANDLES.lock().unwrap();
                let writer = handles.get_mut(&id)
                    .ok_or_else(|| RuntimeError::new(format!("file_write_line: invalid or closed handle {}", id)))?;
                writeln!(writer, "{}", line)
                    .map_err(|e| RuntimeError::new(format!("file_write_line: write error: {}", e)))?;
                Ok(Value::Null)
            }
            "file_close" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("file_close requires 1 argument (handle)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("file_close: handle must be an integer".to_string())),
                };
                let removed_r = READ_HANDLES.lock().unwrap().remove(&id).is_some();
                let removed_w = {
                    let mut handles = WRITE_HANDLES.lock().unwrap();
                    if let Some(writer) = handles.remove(&id) {
                        // Flush before dropping
                        drop(writer);
                        true
                    } else {
                        false
                    }
                };
                if removed_r || removed_w {
                    Ok(Value::Null)
                } else {
                    Err(RuntimeError::new(format!("file_close: invalid or already closed handle {}", id)))
                }
            }
            "csv_write" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "csv_write requires 2 arguments (path, rows)".to_string(),
                    ));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(&PermissionResource::FileSystem("write".to_string()), None)?;
                }
                let path = match &args[0] {
                    Value::String(p) => p.to_string(),
                    _ => return Err(RuntimeError::new("csv_write: path must be a string".to_string())),
                };
                let rows = match &args[1] {
                    Value::Array(rows) => rows.clone(),
                    _ => return Err(RuntimeError::new("csv_write: rows must be an array of arrays".to_string())),
                };
                let csv_str = Self::rows_to_csv(&rows)?;
                std::fs::write(&path, csv_str)
                    .map_err(|e| RuntimeError::new(format!("csv_write: cannot write '{}': {}", path, e)))?;
                Ok(Value::Null)
            }
            "temp_file" => {
                if !args.is_empty() {
                    return Err(RuntimeError::new(
                        "temp_file takes no arguments".to_string(),
                    ));
                }
                // Use tempfile crate for atomic, OS-guaranteed unique temp file (2.7).
                // This eliminates the TOCTOU window present in the old pid+nanos approach.
                let tmp = tempfile::NamedTempFile::new().map_err(|e| {
                    RuntimeError::new(format!("Failed to create temp file: {}", e))
                })?;
                // persist() keeps the file alive after the NamedTempFile is dropped.
                let path = tmp.into_temp_path();
                let path_str = path.to_string_lossy().to_string();
                // Keep the file on disk — caller is responsible for deletion.
                path.keep().map_err(|e| {
                    RuntimeError::new(format!("Failed to persist temp file: {}", e))
                })?;
                Ok(Value::String(Arc::from(path_str)))
            }
            "watch_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "watch_file requires 1 argument (path)".to_string(),
                    ));
                }
                match &args[0] {
                    Value::String(path) => {
                        let mut map = indexmap::IndexMap::new();
                        map.insert("path".to_string(), Value::String(Arc::from(path.clone())));
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
                        // Validate target path to prevent symlink-based path traversal (2.3)
                        let validated_target = Self::validate_path(target)?;
                        if let Some(checker) = permission_checker {
                            use crate::runtime::permissions::PermissionResource;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(target.as_ref()),
                            )?;
                            checker.check_permission(
                                &PermissionResource::FileSystem("write".to_string()),
                                Some(link_path.as_ref()),
                            )?;
                        }
                        let validated_link = Self::validate_path(link_path)?;
                        #[cfg(unix)]
                        {
                            std::os::unix::fs::symlink(&validated_target, &validated_link)
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
                #[cfg(not(feature = "stdlib-full"))]
                return Err(RuntimeError::new(
                    "zip_create requires the 'stdlib-full' feature. \
                     Rebuild with: cargo build --features stdlib-full"
                        .to_string(),
                ));
                #[cfg(feature = "stdlib-full")]
                {
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
                    let validated_output = Self::validate_path(&output_path)?;
                    let file = std::fs::File::create(&validated_output).map_err(|e| {
                        RuntimeError::new(format!(
                            "zip_create: cannot create {}: {}",
                            output_path, e
                        ))
                    })?;
                    let mut zip_writer = zip::ZipWriter::new(file);
                    let options = zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated);
                    let file_args: Vec<String> = if args.len() == 2 {
                        if let Value::Array(arr) = &args[1] {
                            arr.iter()
                                .filter_map(|v| {
                                    if let Value::String(s) = v {
                                        Some(s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else if let Value::String(s) = &args[1] {
                            vec![s.to_string()]
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
                                    Some(s.to_string())
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
                            .unwrap_or(src_path.as_ref());
                        zip_writer.start_file(name, options).map_err(|e| {
                            RuntimeError::new(format!(
                                "zip_create: error adding {}: {}",
                                src_path, e
                            ))
                        })?;
                        let data = std::fs::read(src_path).map_err(|e| {
                            RuntimeError::new(format!(
                                "zip_create: cannot read {}: {}",
                                src_path, e
                            ))
                        })?;
                        use std::io::Write;
                        zip_writer.write_all(&data).map_err(|e| {
                            RuntimeError::new(format!("zip_create: write error: {}", e))
                        })?;
                    }
                    zip_writer.finish().map_err(|e| {
                        RuntimeError::new(format!("zip_create: finalize error: {}", e))
                    })?;
                    Ok(Value::String(Arc::from(validated_output.to_string_lossy().to_string())))
                }
            }
            "zip_extract" => {
                #[cfg(not(feature = "stdlib-full"))]
                return Err(RuntimeError::new(
                    "zip_extract requires the 'stdlib-full' feature. \
                     Rebuild with: cargo build --features stdlib-full"
                        .to_string(),
                ));
                #[cfg(feature = "stdlib-full")]
                {
                    if args.len() < 2 {
                        return Err(RuntimeError::new(
                            "zip_extract requires 2 arguments: archive_path, output_dir"
                                .to_string(),
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
                        Value::String(s) => s.to_string(),
                        _ => {
                            return Err(RuntimeError::new(
                                "zip_extract: output_dir must be a string".to_string(),
                            ))
                        }
                    };
                    let file = std::fs::File::open(archive_path.as_ref()).map_err(|e| {
                        RuntimeError::new(format!(
                            "zip_extract: cannot open {}: {}",
                            archive_path, e
                        ))
                    })?;
                    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
                        RuntimeError::new(format!("zip_extract: invalid archive: {}", e))
                    })?;
                    std::fs::create_dir_all(&output_dir).map_err(|e| {
                        RuntimeError::new(format!(
                            "zip_extract: cannot create output dir: {}",
                            e
                        ))
                    })?;
                    let output_dir_path = std::path::Path::new(&output_dir);
                    let count = archive.len();
                    for i in 0..count {
                        let mut entry = archive.by_index(i).map_err(|e| {
                            RuntimeError::new(format!(
                                "zip_extract: error reading entry {}: {}",
                                i, e
                            ))
                        })?;
                        let entry_name = entry.name().to_string();
                        // Zip-slip prevention: validate entry path against output_dir (0.1)
                        let out_path =
                            Self::validate_zip_entry_path(output_dir_path, &entry_name)?;
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
            }
            // ── Task 15.4: Async File I/O ────────────────────────────────────────
            // Runs synchronous file I/O on a background OS thread and returns
            // a Value::Future. Permission checks happen synchronously before spawn.
            "async_read_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(
                        "async_read_file requires 1 argument (path)".to_string(),
                    ));
                }
                let path = match &args[0] {
                    Value::String(p) => p.clone(),
                    _ => return Err(RuntimeError::new(
                        "async_read_file requires a string path".to_string(),
                    )),
                };
                // Permission check on calling thread
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::FileSystem("read".to_string()),
                        Some(path.as_ref()),
                    )?;
                }
                let validated = Self::validate_path(&path)?;
                let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
                std::thread::spawn(move || {
                    let result = fs::read_to_string(&validated)
                        .map(|s| Value::String(Arc::from(s)))
                        .map_err(|e| format!("Failed to read file: {}", e));
                    sender.send(result);
                });
                Ok(Value::Future(handle))
            }
            "async_write_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(
                        "async_write_file requires 2 arguments (path, content)".to_string(),
                    ));
                }
                let (path, content) = match (&args[0], &args[1]) {
                    (Value::String(p), Value::String(c)) => (p.to_string(), c.to_string()),
                    _ => return Err(RuntimeError::new(
                        "async_write_file requires string path and content".to_string(),
                    )),
                };
                // Permission check on calling thread
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::FileSystem("write".to_string()),
                        Some(path.as_str()),
                    )?;
                }
                let validated = Self::validate_path(path.as_str())?;
                let (handle, sender) = crate::runtime::core::value::FutureHandle::pending();
                std::thread::spawn(move || {
                    let result = fs::write(&validated, content.as_bytes())
                        .map(|_| Value::Null)
                        .map_err(|e| format!("Failed to write file: {}", e));
                    sender.send(result);
                });
                Ok(Value::Future(handle))
            }
            // ── Group 27.4: CSV Streaming ─────────────────────────────────────────

            // csv_stream_reader(path [, delimiter]) → handle_id
            // Opens a CSV file for row-by-row reading.
            "csv_stream_reader" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "csv_stream_reader requires 1 argument (path)".to_string(),
                    ));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    if let Value::String(path) = &args[0] {
                        checker.check_permission(
                            &PermissionResource::FileSystem("read".to_string()),
                            Some(path.as_ref()),
                        )?;
                    }
                }
                let path = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new(
                        "csv_stream_reader: path must be a string".to_string(),
                    )),
                };
                let delimiter = match args.get(1) {
                    Some(Value::String(d)) if !d.is_empty() => {
                        d.chars().next().unwrap_or(',')
                    }
                    _ => ',',
                };
                let file = fs::File::open(&path).map_err(|e| {
                    RuntimeError::new(format!("csv_stream_reader: cannot open '{}': {}", path, e))
                })?;
                let reader = BufReader::new(file);
                let id = next_handle_id();
                CSV_READ_HANDLES.lock().unwrap().insert(id, (reader, delimiter));
                Ok(Value::Integer(id))
            }

            // csv_stream_writer(path [, delimiter]) → handle_id
            // Opens a CSV file for row-by-row writing (overwrites).
            "csv_stream_writer" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "csv_stream_writer requires 1 argument (path)".to_string(),
                    ));
                }
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    if let Value::String(path) = &args[0] {
                        checker.check_permission(
                            &PermissionResource::FileSystem("write".to_string()),
                            Some(path.as_ref()),
                        )?;
                    }
                }
                let path = match &args[0] {
                    Value::String(s) => s.to_string(),
                    _ => return Err(RuntimeError::new(
                        "csv_stream_writer: path must be a string".to_string(),
                    )),
                };
                let delimiter = match args.get(1) {
                    Some(Value::String(d)) if !d.is_empty() => {
                        d.chars().next().unwrap_or(',')
                    }
                    _ => ',',
                };
                let file = fs::File::create(&path).map_err(|e| {
                    RuntimeError::new(format!("csv_stream_writer: cannot create '{}': {}", path, e))
                })?;
                let writer = std::io::BufWriter::new(file);
                let id = next_handle_id();
                CSV_WRITE_HANDLES.lock().unwrap().insert(id, (writer, delimiter));
                Ok(Value::Integer(id))
            }

            // csv_read_row(handle_id) → Array[String] | null (null = EOF)
            "csv_read_row" => {
                let id = match args.first() {
                    Some(Value::Integer(n)) => *n,
                    _ => return Err(RuntimeError::new(
                        "csv_read_row(id): expected integer handle".to_string(),
                    )),
                };
                let mut handles = CSV_READ_HANDLES.lock().unwrap();
                let (reader, delimiter) = handles.get_mut(&id).ok_or_else(|| {
                    RuntimeError::new(format!("csv_read_row: no open reader with id {}", id))
                })?;
                let delim = *delimiter;
                let mut line = String::new();
                let n = reader.read_line(&mut line).map_err(|e| {
                    RuntimeError::new(format!("csv_read_row: read error: {}", e))
                })?;
                if n == 0 {
                    return Ok(Value::Null); // EOF
                }
                let line = line.trim_end_matches('\n').trim_end_matches('\r');
                // Simple CSV parsing (handles quoted fields)
                let fields = Self::parse_csv_line(line, delim);
                Ok(Value::Array(fields.into_iter().map(|s| Value::String(Arc::from(s))).collect()))
            }

            // csv_write_row(handle_id, row_array) → null
            "csv_write_row" => {
                if args.len() < 2 {
                    return Err(RuntimeError::new(
                        "csv_write_row requires 2 arguments (id, row)".to_string(),
                    ));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new(
                        "csv_write_row: first argument must be integer handle".to_string(),
                    )),
                };
                let fields = match &args[1] {
                    Value::Array(arr) => arr.clone(),
                    _ => return Err(RuntimeError::new(
                        "csv_write_row: second argument must be an array".to_string(),
                    )),
                };
                let mut handles = CSV_WRITE_HANDLES.lock().unwrap();
                let (writer, delimiter) = handles.get_mut(&id).ok_or_else(|| {
                    RuntimeError::new(format!("csv_write_row: no open writer with id {}", id))
                })?;
                let delim = *delimiter;
                let line: Vec<String> = fields.iter().map(|f| {
                    let s: String = match f {
                        Value::String(s) => s.to_string(),
                        other => other.to_string(),
                    };
                    if s.contains(delim) || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s
                    }
                }).collect();
                writeln!(writer, "{}", line.join(&delim.to_string())).map_err(|e| {
                    RuntimeError::new(format!("csv_write_row: write error: {}", e))
                })?;
                Ok(Value::Null)
            }

            // csv_stream_close(handle_id) → null
            "csv_stream_close" => {
                let id = match args.first() {
                    Some(Value::Integer(n)) => *n,
                    _ => return Err(RuntimeError::new(
                        "csv_stream_close(id): expected integer handle".to_string(),
                    )),
                };
                // Try to remove from both maps (writer needs flush)
                let removed_writer = CSV_WRITE_HANDLES.lock().unwrap().remove(&id);
                if let Some((mut writer, _)) = removed_writer {
                    writer.flush().ok();
                }
                CSV_READ_HANDLES.lock().unwrap().remove(&id);
                Ok(Value::Null)
            }

            _ => Err(RuntimeError::new(format!("Unknown I/O function: {}", name))),
        }
    }

    /// Parse a single CSV line respecting quoted fields.
    fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                if in_quotes {
                    // Check for escaped quote ("")
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        current.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            } else if c == delimiter && !in_quotes {
                fields.push(current.clone());
                current.clear();
            } else {
                current.push(c);
            }
        }
        fields.push(current);
        fields
    }

    fn rows_to_csv(rows: &[Value]) -> Result<String, RuntimeError> {
        let mut output = String::new();
        for row in rows {
            match row {
                Value::Array(fields) => {
                    let parts: Vec<String> = fields.iter().map(|f| {
                        let s: String = match f {
                            Value::String(s) => s.to_string(),
                            other => other.to_string(),
                        };
                        if s.contains(',') || s.contains('"') || s.contains('\n') {
                            format!("\"{}\"", s.replace('"', "\"\""))
                        } else {
                            s
                        }
                    }).collect();
                    output.push_str(&parts.join(","));
                    output.push('\n');
                }
                _ => return Err(RuntimeError::new("csv_write: each row must be an array".to_string())),
            }
        }
        Ok(output)
    }
}
