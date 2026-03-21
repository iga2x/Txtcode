use crate::runtime::{RuntimeError, Value};
use indexmap::IndexMap;

/// SQLite database library
pub struct DbLib;

#[cfg(feature = "db")]
use std::sync::Mutex;

// ── Connection registry ───────────────────────────────────────────────────────
// SQLite connections are not Send, so we store them as raw pointers behind a Mutex
// keyed by an integer handle ID.  The registry lives for the process lifetime.

#[cfg(feature = "db")]
lazy_static::lazy_static! {
    static ref DB_CONNECTIONS: Mutex<std::collections::HashMap<i64, rusqlite::Connection>> =
        Mutex::new(std::collections::HashMap::new());
    static ref DB_NEXT_ID: Mutex<i64> = Mutex::new(1);
}

impl DbLib {
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        match name {
            // db_open(path) → Integer(id)
            // Requires fs.write permission on path (SQLite creates/modifies the file).
            "db_open" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("db_open requires 1 argument (path)".to_string()));
                }
                let path = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("db_open: path must be a string".to_string())),
                };
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::FileSystem("write".to_string()),
                        Some(path.as_str()),
                    )?;
                }
                Self::db_open_impl(&path)
            }

            // db_exec(id, sql, params?) → Array<Map<String, Value>>
            // params is an optional array of values bound to ? placeholders.
            "db_exec" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "db_exec requires 2-3 arguments (id, sql, params?)".to_string(),
                    ));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("db_exec: id must be an integer".to_string())),
                };
                let sql = match &args[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::new("db_exec: sql must be a string".to_string())),
                };
                let params: Vec<Value> = if args.len() == 3 {
                    match &args[2] {
                        Value::Array(a) => a.clone(),
                        _ => return Err(RuntimeError::new(
                            "db_exec: params must be an array".to_string(),
                        )),
                    }
                } else {
                    Vec::new()
                };
                Self::db_exec_impl(id, &sql, &params)
            }

            // db_close(id) → Null
            "db_close" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("db_close requires 1 argument (id)".to_string()));
                }
                let id = match &args[0] {
                    Value::Integer(n) => *n,
                    _ => return Err(RuntimeError::new("db_close: id must be an integer".to_string())),
                };
                Self::db_close_impl(id)
            }

            _ => Err(RuntimeError::new(format!("Unknown db function: {}", name))),
        }
    }

    // ── Implementation ────────────────────────────────────────────────────────

    #[cfg(feature = "db")]
    fn db_open_impl(path: &str) -> Result<Value, RuntimeError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| RuntimeError::new(format!("db_open: failed to open '{}': {}", path, e)))?;
        let id = {
            let mut next = DB_NEXT_ID.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };
        DB_CONNECTIONS.lock().unwrap().insert(id, conn);
        Ok(Value::Integer(id))
    }

    #[cfg(not(feature = "db"))]
    fn db_open_impl(_path: &str) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "db_open requires the 'db' feature. Rebuild with: cargo build --features db".to_string(),
        ))
    }

    #[cfg(feature = "db")]
    fn db_exec_impl(id: i64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
        use rusqlite::types::ToSql;

        let conns = DB_CONNECTIONS.lock().unwrap();
        let conn = conns.get(&id).ok_or_else(|| {
            RuntimeError::new(format!("db_exec: no open connection with id {}", id))
        })?;

        // Convert Value params to rusqlite ToSql boxes.
        let sql_params: Vec<Box<dyn ToSql>> = params
            .iter()
            .map(|v| -> Box<dyn ToSql> {
                match v {
                    Value::Integer(n) => Box::new(*n),
                    Value::Float(f) => Box::new(*f),
                    Value::String(s) => Box::new(s.clone()),
                    Value::Boolean(b) => Box::new(*b as i64),
                    Value::Null => Box::new(rusqlite::types::Null),
                    other => Box::new(other.to_string()),
                }
            })
            .collect();

        let param_refs: Vec<&dyn ToSql> = sql_params.iter().map(|b| b.as_ref()).collect();

        let mut stmt = conn.prepare(sql).map_err(|e| {
            RuntimeError::new(format!("db_exec: SQL prepare error: {}", e))
        })?;

        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows_result: Result<Vec<Value>, _> = stmt
            .query_map(param_refs.as_slice(), |row| {
                let mut map = IndexMap::new();
                for (i, col_name) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i)?;
                    let npl_val = match val {
                        rusqlite::types::Value::Null => Value::Null,
                        rusqlite::types::Value::Integer(n) => Value::Integer(n),
                        rusqlite::types::Value::Real(f) => Value::Float(f),
                        rusqlite::types::Value::Text(s) => Value::String(s),
                        rusqlite::types::Value::Blob(b) => {
                            Value::String(hex::encode(b))
                        }
                    };
                    map.insert(col_name.clone(), npl_val);
                }
                Ok(Value::Map(map))
            })
            .map_err(|e| RuntimeError::new(format!("db_exec: query error: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RuntimeError::new(format!("db_exec: row error: {}", e)));

        Ok(Value::Array(rows_result?))
    }

    #[cfg(not(feature = "db"))]
    fn db_exec_impl(_id: i64, _sql: &str, _params: &[Value]) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "db_exec requires the 'db' feature.".to_string(),
        ))
    }

    #[cfg(feature = "db")]
    fn db_close_impl(id: i64) -> Result<Value, RuntimeError> {
        DB_CONNECTIONS.lock().unwrap().remove(&id);
        Ok(Value::Null)
    }

    #[cfg(not(feature = "db"))]
    fn db_close_impl(_id: i64) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "db_close requires the 'db' feature.".to_string(),
        ))
    }
}
