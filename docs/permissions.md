# Txt-code Permission and Capability System

Txt-code enforces a layered security model before any privileged operation runs:

```
Intent check → Capability token → Rate limit → Permission grant → Audit log
```

Each layer must pass; the first failure raises a `RuntimeError` and writes a
`Denied` entry to the audit trail.

---

## 1. Permission Resources

The `PermissionResource` type identifies what is being accessed.

| String form | Meaning | Scope example |
|---|---|---|
| `fs.read` | Read files, check existence, list dirs, `file_open`/`file_read_line`/`file_close` | `/var/log/*` |
| `fs.write` | Write, append, copy, move, create dirs, `csv_write`, `file_write_line` | `/tmp/*` |
| `fs.delete` | Delete files and directories | `/tmp/*` |
| `net.connect` | Outbound HTTP, TCP, UDP, DNS, `http_get`, `http_post`, `http_serve` | `*.example.com` |
| `sys.exec` | `exec()`, `exec_json()`, `exec_lines()`, `exec_status()`, `exec_pipe()`, `spawn()`, `kill()`, `signal_send()` | — |
| `sys.env` | `getenv()`, `setenv()`, `env_list()` | — |
| `sys.info` | `cpu_count()`, `memory_available()`, `disk_space()` | — |

Aliases: `filesystem` = `fs`, `network` = `net`, `system` = `sys`, `proc` = `process`.

> **Removed in v0.4.1**: `wifi.*` and `ble.*` permission resources have been removed.
> Attempting to use `wifi.*` or `ble.*` strings now returns a clear error.

---

## 2. Granting Permissions

### At the CLI (before execution)

```bash
# Allow reads under /var/log only
txtcode run scan.tc --allow-fs=/var/log

# Allow outbound to one host pattern
txtcode run probe.tc --allow-net=api.example.com

# Deny all privileged access — safest mode
txtcode run untrusted.tc --sandbox

# Print all permissions the script would request, then exit (no execution)
txtcode run untrusted.tc --permissions-report

# Permissions report as JSON (for CI / tooling)
txtcode run untrusted.tc --permissions-report --json
```

`--sandbox` is equivalent to denying all resources. Individual `--allow-*`
flags still apply on top.

`--permissions-report` is a read-only inspection mode: the script is parsed
but never executed. It prints each permission string and the function calls
that would request it.

### In a script

```txtcode
grant_permission("fs.read",    "/tmp/*")      # scoped to /tmp subtree
grant_permission("net.connect", "*.corp.lan") # scoped to corp.lan domain
grant_permission("sys.exec",   null)          # allow all exec calls
```

Grants added in a script are cumulative. A `deny_permission` call overrides
any grant:

```txtcode
deny_permission("sys.exec", null)   # no exec, even if previously granted
```

### In env.toml

```toml
[permissions]
granted = ["fs.read:/data/*", "net.connect:*.api.io", "sys.info"]
denied  = ["sys.exec", "fs.delete"]
```

---

## 3. Scope Matching (Glob Patterns)

When a scope is provided, the permission engine matches the actual resource
path or hostname against the pattern using glob rules:

- `*` matches any sequence of characters (including `/` in paths).
- A permission with **no scope** matches any resource.
- A permission with scope `"/tmp/*"` matches `/tmp/file.txt` but not `/var/tmp/file.txt`.

```txtcode
grant_permission("fs.read", "/var/log/*")  # matches /var/log/syslog
grant_permission("net.connect", "*.corp")  # matches db.corp, api.corp
```

---

## 4. Capability Tokens

Capability tokens are short-lived, explicitly revocable grants backed by the
`CapabilityManager`. They are the preferred pattern when a block of code
needs a permission that should not persist for the whole script.

```txtcode
# Acquire token
store → tok → grant_capability("wifi.capture", null)

# Activate token for the current scope
use_capability(tok)

# All wifi_capture() calls here check against the token
store → frames → wifi_capture("wlan0")

# Revoke when done — subsequent calls fail even within the same scope
revoke_capability(tok)
```

Capability functions:

| Function | Description |
|---|---|
| `grant_capability(cap, scope)` | Issue a new capability token |
| `use_capability(token_id)` | Activate a token as the current capability |
| `revoke_capability(token_id)` | Revoke a token immediately |
| `capability_valid(token_id)` | Returns `true` if the token is active and not expired |

---

## 5. Function-Level Declarations

Functions declare their intent and required capabilities inside the function body,
before any other statements:

```txtcode
define → scan_ports → (host: string) → array
  intent   → "TCP reachability probe only"
  allowed  → ["net.connect"]
  forbidden → ["sys.exec", "fs.write"]

  store → open_ports → []
  for → port in [22, 80, 443, 3389, 8080]
    if → is_ok(tcp_connect(f"{host}:{port}"))
      open_ports += [port]
    end
  end
  return → open_ports
end
```

### Enforcement rules

| Declaration | When checked | Effect on violation |
|---|---|---|
| `forbidden → ["cap"]` | **Validation time** (before execution) | `ValidationError` — script never starts |
| `allowed → ["cap"]` | Audit time (runtime) | Logged as advisory; execution continues |
| `intent → "..."` | Runtime (per privileged call) | `intent.violation.*` in audit trail |

A function body that calls `exec()` while declaring `forbidden → ["sys.exec"]`
is **caught by the validator** — the script exits before any code runs.

---

## 6. Audit Trail

Every permission check is logged regardless of outcome.

```
[2026-03-18 10:12:01.847ms] fs.read        scope="/var/log/*"  ALLOWED  permission:grant
[2026-03-18 10:12:01.851ms] net.connect    scope="10.0.0.1"    DENIED   Permission not granted: net.connect
[2026-03-18 10:12:01.853ms] security.startup level=full platform=linux secure=true
```

Each entry includes:
- Monotonic nanosecond timestamp
- Action category (`fs.read`, `net.connect`, `sys.exec`, etc.)
- Scope value (path, hostname, etc.)
- Result: `Allowed` or `Denied`
- Source: `capability:<id>`, permission grant, or intent violation

---

## 7. Permission Check Order

When a privileged call (e.g. `exec_json()`, `read_file()`, `http_get()`) is invoked,
the runtime runs these steps in order:

1. **Max execution time** — if exceeded, fail immediately.
2. **Intent check** — if the enclosing function has an `intent` or `allowed`
   declaration, verify the action is permitted.
3. **Capability token** — if an active token covers this resource, check it.
   If the token is valid and no explicit `deny` overrides it, allow and log.
4. **Rate limit** — check the policy engine rate limit for this resource.
5. **Permission grant** — check `PermissionManager` for a matching grant.
6. **Log result** — write `Allowed` or `Denied` to the audit trail.

---

## 8. Error Messages

| Situation | Error text |
|---|---|
| No grant for the resource | `Permission not granted: fs.read` |
| Resource explicitly denied | `Permission denied: sys.exec` |
| Forbidden capability called | `Function 'fn' forbids 'sys.exec' but its body calls 'exec'` |
| Capability token not found | `Capability denied: capability token not found` |
| Capability token revoked | `Capability denied: capability token 'cap_abc' has been revoked` |
| Capability token expired | `Capability denied: capability token 'cap_abc' has expired` |
| Unknown resource string | `Permission resource 'wifi.scan' is not supported. WiFi/Bluetooth capabilities were removed in v0.4.1.` |
| Intent violation | `intent.violation.net.connect` (audit trail; not a hard error) |
| Rate limit exceeded | `Rate limit exceeded for net.connect: 100 per 3600s` |
| Struct field type mismatch | `Struct field type mismatch: 'Point.x' expected Int, got string` (E0016) |
| Async without experimental | `[WARNING] async function 'fn': executes synchronously (E0051)` |

---

## 9. Security Level

The security level is auto-selected from available platform capabilities:

| Level | Active checks |
|---|---|
| `none` | No platform checks available |
| `basic` | Timing micro-benchmark + environment injection scan |
| `standard` | Timing + OS-level debugger detection (Linux: TracerPid, wchan, parent name) |
| `full` | Standard + source integrity hash verification |

On a normal Linux `txtcode run` invocation the level is **full**.

See [Security Features](security-features.md) for implementation details.
