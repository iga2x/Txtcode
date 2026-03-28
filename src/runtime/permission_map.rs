/// Central permission mapping for stdlib functions.
///
/// This module is the single source of truth for the function-name →
/// PermissionResource relationship. Both call sites that enforce permissions
/// use these helpers:
///
///  - `runtime/execution/expressions/function_calls.rs` — per-call gate with
///    scope extracted from runtime arguments.
///  - `stdlib/mod.rs` — upfront gate (scope = None) before mutable dispatch.
///
/// Adding a new privileged function means updating `map_function_to_permission`
/// only. Both enforcement points pick it up automatically.
use crate::runtime::core::Value;
use crate::runtime::permissions::PermissionResource;

/// Returns the `PermissionResource` required to call `name`, or `None` if the
/// function is unprivileged and needs no permission check.
///
/// The returned resource carries the action string (e.g. `"read"`, `"write"`,
/// `"connect"`) but NOT the scope; callers that have runtime arguments should
/// pass them to [`extract_permission_scope`] to obtain a concrete scope value.
pub fn map_function_to_permission(name: &str) -> Option<PermissionResource> {
    match name {
        // ── Filesystem reads ──────────────────────────────────────────────
        "read_file"
        | "read_lines"
        | "read_file_binary"
        | "file_exists"
        | "is_file"
        | "is_dir"
        | "list_dir"
        | "watch_file" => Some(PermissionResource::FileSystem("read".to_string())),

        // ── Filesystem writes ─────────────────────────────────────────────
        "write_file"
        | "write_file_binary"
        | "append_file"
        | "copy_file"
        | "move_file"
        | "rename_file"
        | "temp_file"
        | "symlink_create"
        | "mkdir"
        | "csv_write"
        | "zip_create"
        | "zip_extract" => Some(PermissionResource::FileSystem("write".to_string())),

        // ── Filesystem deletes ────────────────────────────────────────────
        "delete" | "rmdir" => Some(PermissionResource::FileSystem("delete".to_string())),

        // ── Network connections ───────────────────────────────────────────
        "http_get"
        | "http_post"
        | "http_put"
        | "http_delete"
        | "http_patch"
        | "async_http_get"
        | "async_http_post"
        | "tcp_connect"
        | "udp_send"
        | "resolve" => Some(PermissionResource::Network("connect".to_string())),

        // ── Process execution ─────────────────────────────────────────────
        // `kill` and `tool_exec` gate on System("exec"); per-tool whitelisting
        // is enforced separately inside ToolExecutor / SysLib.
        "exec" | "exec_status" | "exec_lines" | "exec_json"
        | "spawn" | "kill" | "pipe_exec" | "tool_exec" => {
            Some(PermissionResource::System("exec".to_string()))
        }

        // ── System environment ────────────────────────────────────────────
        "getenv" | "setenv" | "env_list" => {
            Some(PermissionResource::System("env".to_string()))
        }

        // ── System info (read-only) ───────────────────────────────────────
        "cpu_count"
        | "memory"
        | "memory_available"
        | "disk_space"
        | "platform"
        | "arch"
        | "pid"
        | "user"
        | "uid"
        | "gid"
        | "is_root"
        | "os_name"
        | "os_version" => Some(PermissionResource::System("info".to_string())),

        // ── Process signals ───────────────────────────────────────────────
        "signal_send" => {
            Some(PermissionResource::Process(vec!["signal_send".to_string()]))
        }

        // ── Database connections (multi-driver) ───────────────────────────
        "db_connect" | "db_query" | "db_execute" | "db_transaction" | "db_commit" | "db_rollback" => {
            Some(PermissionResource::System("db".to_string()))
        }

        // ── FFI — native shared-library loading ───────────────────────────
        "ffi_load" | "ffi_call" | "ffi_close" => {
            Some(PermissionResource::System("ffi".to_string()))
        }

        // ── Plugin — higher-level native plugin system ────────────────────
        "plugin_load" | "plugin_functions" | "plugin_call" => {
            Some(PermissionResource::System("ffi".to_string()))
        }

        // ── WASM execution (Task 29.3) ────────────────────────────────────
        "wasm_load" | "wasm_call" | "wasm_close" => {
            Some(PermissionResource::System("ffi".to_string()))
        }

        _ => None,
    }
}

/// Extracts a concrete scope string from call arguments based on the resource type.
///
/// | Resource type          | Scope source                                    |
/// |------------------------|-------------------------------------------------|
/// | `FileSystem(_)`        | First string argument (file path)               |
/// | `Network(_)`           | Hostname parsed from URL in first string arg    |
/// | `System("exec")`       | First whitespace-token of command in first arg  |
/// | Everything else        | `None` — no scope required                      |
///
/// Returns `None` when the first argument is absent, not a string, or (for
/// network functions) produces an empty hostname.  Call-sites that require a
/// concrete scope (FileSystem, Network, exec) should skip the permission check
/// when this returns `None`.
pub fn extract_permission_scope(resource: &PermissionResource, args: &[Value]) -> Option<String> {
    let first_string = || {
        args.first().and_then(|v| match v {
            Value::String(s) => Some(s.to_string()),
            _ => None,
        })
    };

    match resource {
        PermissionResource::FileSystem(_) => first_string(),

        PermissionResource::Network(_) => args.first().and_then(|v| match v {
            Value::String(url) => {
                let host = url
                    .split("//")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .and_then(|s| s.split(':').next())
                    .unwrap_or(url.as_ref());
                if host.is_empty() {
                    None
                } else {
                    Some(host.to_string())
                }
            }
            _ => None,
        }),

        PermissionResource::System(action) if action == "exec" => args.first().and_then(|v| {
            match v {
                Value::String(cmd) => cmd.split_whitespace().next().map(|s| s.to_string()),
                _ => None,
            }
        }),

        // System("env"), System("info"), Process: no scope.
        _ => None,
    }
}

/// Returns true for resources where a concrete scope (extracted from call
/// arguments) is required before the permission check should proceed.
///
/// When this returns `true` and [`extract_permission_scope`] returns `None`,
/// the caller should skip the permission check — the target is unknown.
pub fn resource_requires_scope(resource: &PermissionResource) -> bool {
    match resource {
        PermissionResource::FileSystem(_) | PermissionResource::Network(_) => true,
        PermissionResource::System(action) => action == "exec",
        _ => false,
    }
}
