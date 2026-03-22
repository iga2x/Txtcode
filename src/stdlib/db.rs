use crate::runtime::{RuntimeError, Value};
use indexmap::IndexMap;

/// Database library — supports SQLite (feature "db"), PostgreSQL (feature "postgres"),
/// and MySQL (feature "mysql").
pub struct DbLib;

// ── Connection registry ───────────────────────────────────────────────────────
// A unified enum wraps each driver's connection type so all handles share one
// integer-keyed registry.

#[allow(dead_code)]
enum DbConnection {
    #[cfg(feature = "db")]
    Sqlite(rusqlite::Connection),
    #[cfg(feature = "postgres")]
    Postgres(Box<postgres::Client>),
    #[cfg(feature = "mysql")]
    Mysql(mysql::Conn),
    /// Placeholder when no db feature is active — never instantiated.
    Unsupported,
}

// SAFETY: rusqlite::Connection, postgres::Client, and mysql::Conn are all Send.
unsafe impl Send for DbConnection {}

use std::sync::Mutex;
use std::sync::Arc;

lazy_static::lazy_static! {
    static ref DB_CONNECTIONS: Mutex<std::collections::HashMap<i64, DbConnection>> =
        Mutex::new(std::collections::HashMap::new());
    static ref DB_NEXT_ID: Mutex<i64> = Mutex::new(1);
}

fn next_id() -> i64 {
    let mut n = DB_NEXT_ID.lock().unwrap();
    let id = *n;
    *n += 1;
    id
}

impl DbLib {
    pub fn call_function(
        name: &str,
        args: &[Value],
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        match name {
            // ── Legacy SQLite API (backward compat) ───────────────────────────
            // db_open(path) → int
            "db_open" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("db_open requires 1 argument (path)".to_string()));
                }
                let path = str_arg(args, 0, "db_open")?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::FileSystem("write".to_string()),
                        Some(path.as_str()),
                    )?;
                }
                Self::open_sqlite(&path)
            }

            // db_exec(id, sql, params?) → array[map]
            "db_exec" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "db_exec requires 2-3 arguments (id, sql, params?)".to_string(),
                    ));
                }
                let id = int_arg(args, 0, "db_exec")?;
                let sql = str_arg(args, 1, "db_exec")?;
                let params = optional_array_arg(args, 2, "db_exec")?;
                Self::query_impl(id, &sql, &params)
            }

            // ── Unified multi-driver API ──────────────────────────────────────
            // db_connect(url) → int
            // url schemes: sqlite:///path  sqlite::memory:  postgres://...  mysql://...
            "db_connect" => {
                if args.is_empty() {
                    return Err(RuntimeError::new("db_connect requires 1 argument (url)".to_string()));
                }
                let url = str_arg(args, 0, "db_connect")?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("db".to_string()),
                        None,
                    )?;
                }
                Self::connect(&url)
            }

            // db_query(conn, sql, params?) → array[map]
            "db_query" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "db_query requires 2-3 arguments (conn, sql, params?)".to_string(),
                    ));
                }
                let id = int_arg(args, 0, "db_query")?;
                let sql = str_arg(args, 1, "db_query")?;
                let params = optional_array_arg(args, 2, "db_query")?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("db".to_string()),
                        None,
                    )?;
                }
                Self::query_impl(id, &sql, &params)
            }

            // db_execute(conn, sql, params?) → int  (rows affected)
            "db_execute" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(RuntimeError::new(
                        "db_execute requires 2-3 arguments (conn, sql, params?)".to_string(),
                    ));
                }
                let id = int_arg(args, 0, "db_execute")?;
                let sql = str_arg(args, 1, "db_execute")?;
                let params = optional_array_arg(args, 2, "db_execute")?;
                if let Some(checker) = permission_checker {
                    use crate::runtime::permissions::PermissionResource;
                    checker.check_permission(
                        &PermissionResource::System("db".to_string()),
                        None,
                    )?;
                }
                Self::execute_impl(id, &sql, &params)
            }

            // db_transaction(conn) → null  — issues BEGIN
            // db_commit(conn) → null       — issues COMMIT
            // db_rollback(conn) → null     — issues ROLLBACK
            //
            // Usage pattern:
            //   db_transaction(conn)
            //   db_execute(conn, "INSERT ...")
            //   db_commit(conn)   OR   db_rollback(conn)
            "db_transaction" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "db_transaction requires 1 argument: db_transaction(conn)".to_string(),
                    ));
                }
                let id = int_arg(args, 0, "db_transaction")?;
                Self::execute_impl(id, "BEGIN", &[])?;
                Ok(Value::Null)
            }

            "db_commit" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "db_commit requires 1 argument: db_commit(conn)".to_string(),
                    ));
                }
                let id = int_arg(args, 0, "db_commit")?;
                Self::execute_impl(id, "COMMIT", &[])?;
                Ok(Value::Null)
            }

            "db_rollback" => {
                if args.is_empty() {
                    return Err(RuntimeError::new(
                        "db_rollback requires 1 argument: db_rollback(conn)".to_string(),
                    ));
                }
                let id = int_arg(args, 0, "db_rollback")?;
                Self::execute_impl(id, "ROLLBACK", &[])?;
                Ok(Value::Null)
            }

            // db_close(id) → null
            "db_close" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new("db_close requires 1 argument (id)".to_string()));
                }
                let id = int_arg(args, 0, "db_close")?;
                DB_CONNECTIONS.lock().unwrap().remove(&id);
                Ok(Value::Null)
            }

            _ => Err(RuntimeError::new(format!("Unknown db function: {}", name))),
        }
    }

    // R.1: db_transaction(conn, handler_fn) — auto-commit/rollback closure API.
    //
    // Issues BEGIN before calling the handler. If the handler returns Ok, issues COMMIT.
    // If the handler returns Err (runtime error), issues ROLLBACK and re-raises the error.
    // This guarantees the transaction is always closed regardless of handler outcome.
    //
    // Backward compat: db_transaction(conn) [no handler] still issues plain BEGIN.
    pub fn transaction_with_executor(
        args: &[Value],
        executor: &mut dyn crate::stdlib::FunctionExecutor,
        permission_checker: Option<&dyn crate::stdlib::permission_checker::PermissionChecker>,
    ) -> Result<Value, RuntimeError> {
        if args.is_empty() {
            return Err(RuntimeError::new(
                "db_transaction requires at least 1 argument: db_transaction(conn[, handler])".to_string(),
            ));
        }
        let id = int_arg(args, 0, "db_transaction")?;
        if let Some(checker) = permission_checker {
            use crate::runtime::permissions::PermissionResource;
            checker.check_permission(&PermissionResource::System("db".to_string()), None)?;
        }

        // If no handler given, fall back to legacy BEGIN-only behaviour.
        let handler = match args.get(1) {
            Some(v @ Value::Function(..)) => v.clone(),
            None | Some(Value::Null) => {
                Self::execute_impl(id, "BEGIN", &[])?;
                return Ok(Value::Null);
            }
            _ => return Err(RuntimeError::new(
                "db_transaction: second argument must be a function (handler closure)".to_string(),
            )),
        };

        // Issue BEGIN
        Self::execute_impl(id, "BEGIN", &[])?;

        // Call the handler
        match executor.call_function_value(&handler, &[]) {
            Ok(val) => {
                // Success → COMMIT (ignore commit error — log but return success)
                let _ = Self::execute_impl(id, "COMMIT", &[]);
                Ok(val)
            }
            Err(e) => {
                // Failure → ROLLBACK (best-effort; ignore rollback error)
                let _ = Self::execute_impl(id, "ROLLBACK", &[]);
                Err(e) // re-raise the original error
            }
        }
    }

    // R.2: Maximum simultaneous open connections.
    const MAX_DB_CONNECTIONS: usize = 50;

    // ── connect() — scheme dispatch ───────────────────────────────────────────

    fn connect(url: &str) -> Result<Value, RuntimeError> {
        // R.2: Enforce connection limit before opening.
        {
            let guard = DB_CONNECTIONS.lock().unwrap();
            if guard.len() >= Self::MAX_DB_CONNECTIONS {
                return Err(RuntimeError::new(format!(
                    "db_connect: maximum {} connections reached; call db_close() to release one",
                    Self::MAX_DB_CONNECTIONS
                )));
            }
        }
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Self::open_postgres(url)
        } else if url.starts_with("mysql://") {
            Self::open_mysql(url)
        } else {
            // sqlite:///path/to/db  OR  sqlite::memory:  OR  bare path
            let path = url
                .strip_prefix("sqlite:///")
                .or_else(|| url.strip_prefix("sqlite:"))
                .unwrap_or(url);
            Self::open_sqlite(path)
        }
    }

    // ── SQLite ────────────────────────────────────────────────────────────────

    #[cfg(feature = "db")]
    fn open_sqlite(path: &str) -> Result<Value, RuntimeError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| RuntimeError::new(format!("db_connect: SQLite open '{}': {}", path, e)))?;
        let id = next_id();
        DB_CONNECTIONS
            .lock()
            .unwrap()
            .insert(id, DbConnection::Sqlite(conn));
        Ok(Value::Integer(id))
    }

    #[cfg(not(feature = "db"))]
    fn open_sqlite(_path: &str) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "SQLite support requires the 'db' feature. Rebuild with: cargo build --features db",
        ))
    }

    // ── PostgreSQL ────────────────────────────────────────────────────────────

    #[cfg(feature = "postgres")]
    fn open_postgres(url: &str) -> Result<Value, RuntimeError> {
        let client = postgres::Client::connect(url, postgres::NoTls)
            .map_err(|e| RuntimeError::new(format!("db_connect: PostgreSQL '{}': {}", url, e)))?;
        let id = next_id();
        DB_CONNECTIONS
            .lock()
            .unwrap()
            .insert(id, DbConnection::Postgres(Box::new(client)));
        Ok(Value::Integer(id))
    }

    #[cfg(not(feature = "postgres"))]
    fn open_postgres(url: &str) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(format!(
            "db_connect: PostgreSQL support requires the 'postgres' feature ('{}').\n\
             Rebuild with: cargo build --features postgres",
            url
        )))
    }

    // ── MySQL ─────────────────────────────────────────────────────────────────

    #[cfg(feature = "mysql")]
    fn open_mysql(url: &str) -> Result<Value, RuntimeError> {
        let opts = mysql::Opts::from_url(url)
            .map_err(|e| RuntimeError::new(format!("db_connect: MySQL URL parse '{}': {}", url, e)))?;
        let conn = mysql::Conn::new(opts)
            .map_err(|e| RuntimeError::new(format!("db_connect: MySQL connect '{}': {}", url, e)))?;
        let id = next_id();
        DB_CONNECTIONS
            .lock()
            .unwrap()
            .insert(id, DbConnection::Mysql(conn));
        Ok(Value::Integer(id))
    }

    #[cfg(not(feature = "mysql"))]
    fn open_mysql(url: &str) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(format!(
            "db_connect: MySQL support requires the 'mysql' feature ('{}').\n\
             Rebuild with: cargo build --features mysql",
            url
        )))
    }

    // ── query_impl — returns Vec<map> ─────────────────────────────────────────

    fn query_impl(id: i64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
        let mut conns = DB_CONNECTIONS.lock().unwrap();
        let conn = conns.get_mut(&id).ok_or_else(|| {
            RuntimeError::new(format!("db_query: no open connection with id {}", id))
        })?;
        match conn {
            #[cfg(feature = "db")]
            DbConnection::Sqlite(c) => sqlite_query(c, sql, params),
            #[cfg(feature = "postgres")]
            DbConnection::Postgres(c) => pg_query(c, sql, params),
            #[cfg(feature = "mysql")]
            DbConnection::Mysql(c) => mysql_query(c, sql, params),
            DbConnection::Unsupported => Err(RuntimeError::new("No database driver available".to_string())),
            #[allow(unreachable_patterns)]
            _ => Err(RuntimeError::new("No database driver available".to_string())),
        }
    }

    // ── execute_impl — returns rows_affected ──────────────────────────────────

    fn execute_impl(id: i64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
        let mut conns = DB_CONNECTIONS.lock().unwrap();
        let conn = conns.get_mut(&id).ok_or_else(|| {
            RuntimeError::new(format!("db_execute: no open connection with id {}", id))
        })?;
        match conn {
            #[cfg(feature = "db")]
            DbConnection::Sqlite(c) => sqlite_execute(c, sql, params),
            #[cfg(feature = "postgres")]
            DbConnection::Postgres(c) => pg_execute(c, sql, params),
            #[cfg(feature = "mysql")]
            DbConnection::Mysql(c) => mysql_execute(c, sql, params),
            DbConnection::Unsupported => Err(RuntimeError::new("No database driver available".to_string())),
            #[allow(unreachable_patterns)]
            _ => Err(RuntimeError::new("No database driver available".to_string())),
        }
    }
}

// ── Argument helpers ──────────────────────────────────────────────────────────

fn str_arg(args: &[Value], idx: usize, fn_name: &str) -> Result<String, RuntimeError> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.to_string()),
        _ => Err(RuntimeError::new(format!(
            "{}: argument {} must be a string",
            fn_name, idx
        ))),
    }
}

fn int_arg(args: &[Value], idx: usize, fn_name: &str) -> Result<i64, RuntimeError> {
    match args.get(idx) {
        Some(Value::Integer(n)) => Ok(*n),
        _ => Err(RuntimeError::new(format!(
            "{}: argument {} must be an integer",
            fn_name, idx
        ))),
    }
}

fn optional_array_arg(
    args: &[Value],
    idx: usize,
    fn_name: &str,
) -> Result<Vec<Value>, RuntimeError> {
    match args.get(idx) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(a)) => Ok(a.clone()),
        _ => Err(RuntimeError::new(format!(
            "{}: params argument must be an array",
            fn_name
        ))),
    }
}

// ── SQLite driver helpers ─────────────────────────────────────────────────────

#[cfg(feature = "db")]
fn sqlite_query(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[Value],
) -> Result<Value, RuntimeError> {
    use rusqlite::types::ToSql;

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

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| RuntimeError::new(format!("db_query: SQL prepare: {}", e)))?;

    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let rows: Vec<Value> = stmt
        .query_map(param_refs.as_slice(), |row| {
            let mut map = IndexMap::new();
            for (i, col_name) in col_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i)?;
                let npl_val = match val {
                    rusqlite::types::Value::Null => Value::Null,
                    rusqlite::types::Value::Integer(n) => Value::Integer(n),
                    rusqlite::types::Value::Real(f) => Value::Float(f),
                    rusqlite::types::Value::Text(s) => Value::String(Arc::from(s)),
                    rusqlite::types::Value::Blob(b) => Value::String(Arc::from(hex::encode(b))),
                };
                map.insert(col_name.clone(), npl_val);
            }
            Ok(Value::Map(map))
        })
        .map_err(|e| RuntimeError::new(format!("db_query: query error: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| RuntimeError::new(format!("db_query: row error: {}", e)))?;

    Ok(Value::Array(rows))
}

#[cfg(feature = "db")]
fn sqlite_execute(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[Value],
) -> Result<Value, RuntimeError> {
    use rusqlite::types::ToSql;

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

    let affected = conn
        .execute(sql, param_refs.as_slice())
        .map_err(|e| RuntimeError::new(format!("db_execute: {}", e)))?;

    Ok(Value::Integer(affected as i64))
}

// ── PostgreSQL driver helpers ─────────────────────────────────────────────────

#[cfg(feature = "postgres")]
fn pg_query(
    client: &mut postgres::Client,
    sql: &str,
    params: &[Value],
) -> Result<Value, RuntimeError> {
    use postgres::types::ToSql;

    let pg_params: Vec<Box<dyn ToSql + Sync>> = params
        .iter()
        .map(|v| -> Box<dyn ToSql + Sync> {
            match v {
                Value::Integer(n) => Box::new(*n),
                Value::Float(f) => Box::new(*f),
                Value::String(s) => Box::new(s.clone()),
                Value::Boolean(b) => Box::new(*b),
                _ => Box::new(Option::<String>::None),
            }
        })
        .collect();
    let param_refs: Vec<&(dyn ToSql + Sync)> =
        pg_params.iter().map(|b| b.as_ref()).collect();

    let rows = client
        .query(sql, param_refs.as_slice())
        .map_err(|e| RuntimeError::new(format!("db_query: PostgreSQL: {}", e)))?;

    let result: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut map = IndexMap::new();
            for (i, col) in row.columns().iter().enumerate() {
                let name = col.name().to_string();
                let val = pg_row_value(row, i);
                map.insert(name, val);
            }
            Value::Map(map)
        })
        .collect();

    Ok(Value::Array(result))
}

#[cfg(feature = "postgres")]
fn pg_row_value(row: &postgres::Row, idx: usize) -> Value {
    // Try common types in order; fall back to string representation.
    if let Ok(Some(v)) = row.try_get::<_, Option<i64>>(idx) {
        return Value::Integer(v);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<i32>>(idx) {
        return Value::Integer(v as i64);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<f64>>(idx) {
        return Value::Float(v);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<bool>>(idx) {
        return Value::Boolean(v);
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
        return npl_value_from_str(v);
    }
    // NULL or unrecognised type
    Value::Null
}

#[cfg(feature = "postgres")]
fn pg_execute(
    client: &mut postgres::Client,
    sql: &str,
    params: &[Value],
) -> Result<Value, RuntimeError> {
    use postgres::types::ToSql;

    let pg_params: Vec<Box<dyn ToSql + Sync>> = params
        .iter()
        .map(|v| -> Box<dyn ToSql + Sync> {
            match v {
                Value::Integer(n) => Box::new(*n),
                Value::Float(f) => Box::new(*f),
                Value::String(s) => Box::new(s.clone()),
                Value::Boolean(b) => Box::new(*b),
                _ => Box::new(Option::<String>::None),
            }
        })
        .collect();
    let param_refs: Vec<&(dyn ToSql + Sync)> =
        pg_params.iter().map(|b| b.as_ref()).collect();

    let affected = client
        .execute(sql, param_refs.as_slice())
        .map_err(|e| RuntimeError::new(format!("db_execute: PostgreSQL: {}", e)))?;

    Ok(Value::Integer(affected as i64))
}

// ── MySQL driver helpers ──────────────────────────────────────────────────────

#[cfg(feature = "mysql")]
fn mysql_query(
    conn: &mut mysql::Conn,
    sql: &str,
    params: &[Value],
) -> Result<Value, RuntimeError> {
    use mysql::prelude::Queryable;

    let mysql_params: mysql::Params = if params.is_empty() {
        mysql::Params::Empty
    } else {
        mysql::Params::Positional(params.iter().map(value_to_mysql).collect())
    };

    let rows: Vec<mysql::Row> = conn
        .exec(sql, mysql_params)
        .map_err(|e| RuntimeError::new(format!("db_query: MySQL: {}", e)))?;

    let result: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut map = IndexMap::new();
            for (i, col) in row.columns_ref().iter().enumerate() {
                let name = col.name_str().to_string();
                let val = mysql_val_to_value(row.as_ref()[i].clone());
                map.insert(name, val);
            }
            Value::Map(map)
        })
        .collect();

    Ok(Value::Array(result))
}

#[cfg(feature = "mysql")]
fn mysql_execute(
    conn: &mut mysql::Conn,
    sql: &str,
    params: &[Value],
) -> Result<Value, RuntimeError> {
    use mysql::prelude::Queryable;

    let mysql_params: mysql::Params = if params.is_empty() {
        mysql::Params::Empty
    } else {
        mysql::Params::Positional(params.iter().map(value_to_mysql).collect())
    };

    conn.exec_drop(sql, mysql_params)
        .map_err(|e| RuntimeError::new(format!("db_execute: MySQL: {}", e)))?;

    let affected = conn.affected_rows();
    Ok(Value::Integer(affected as i64))
}

#[cfg(feature = "mysql")]
fn value_to_mysql(v: &Value) -> mysql::Value {
    match v {
        Value::Integer(n) => mysql::Value::Int(*n),
        Value::Float(f) => mysql::Value::Double(*f),
        Value::String(s) => mysql::Value::Bytes(s.as_bytes().to_vec()),
        Value::Boolean(b) => mysql::Value::Int(*b as i64),
        Value::Null => mysql::Value::NULL,
        other => mysql::Value::Bytes(other.to_string().into_bytes()),
    }
}

#[cfg(feature = "mysql")]
fn mysql_val_to_value(v: mysql::Value) -> Value {
    match v {
        mysql::Value::NULL => Value::Null,
        mysql::Value::Int(n) => Value::Integer(n),
        mysql::Value::UInt(n) => Value::Integer(n as i64),
        mysql::Value::Float(f) => Value::Float(f as f64),
        mysql::Value::Double(f) => Value::Float(f),
        mysql::Value::Bytes(b) => Value::String(Arc::from(String::from_utf8_lossy(&b).into_owned())),
        other => Value::String(Arc::from(format!("{:?}", other))),
    }
}
