# Txt-code Security Features

Txt-code is designed for ethical security research, penetration testing, and
security-aware scripting. Security is enforced at every layer: permissions,
capability tokens, intent declarations, runtime anti-debug checks, script signing,
and an audit trail.

---

## 1. Permission System

Every privileged operation requires an explicit grant before it runs.
Permissions are checked at runtime; a missing grant raises an error and
writes a `denied` entry to the audit trail.

### Permission resources

| Resource string | Protects |
|---|---|
| `fs.read` | Reading files, checking existence, listing directories |
| `fs.write` | Writing, appending, copying, moving, creating dirs |
| `fs.delete` | Deleting files and directories |
| `net.connect` | HTTP requests, TCP/UDP connections, DNS resolution |
| `sys.exec` | `exec()`, `spawn()`, `pipe_exec()`, `kill()`, `signal_send()` |
| `sys.env` | `getenv()`, `setenv()` |
| `wifi.scan` | Passive WiFi interface / probe-response enumeration |
| `wifi.capture` | Raw WiFi frame capture (monitor mode) |
| `wifi.deauth` | Deauthentication frame injection (requires authorisation) |
| `wifi.inject` | Arbitrary WiFi packet injection (requires authorisation) |
| `ble.scan` | BLE advertisement scanning / device discovery |
| `ble.connect` | GATT connection to a remote BLE device |
| `ble.fuzz` | Malformed BLE PDU injection (requires authorisation) |
| `ble.read` | Read GATT characteristic values |
| `ble.write` | Write GATT characteristic values |

### Granting permissions at the CLI

```bash
# Allow filesystem reads under /var/log and network to one host
txtcode run scan.tc --allow-fs=/var/log --allow-net=192.168.1.1

# Deny everything except what is declared (sandbox mode)
txtcode run scan.tc --sandbox
```

### Scope-limited grants in scripts

```txtcode
# Grant can be scoped with glob patterns
grant_permission("fs.read",  "/tmp/*")
grant_permission("net.connect", "*.example.com")
grant_permission("wifi.scan", null)
```

### Permission declarations in functions

Functions may declare their expected permissions. The validator enforces
that a `forbidden` declaration is never violated:

```txtcode
define → safe_fetch → (url: string)
  allowed → ["net.connect"]
  forbidden → ["sys.exec", "fs.write"]
  return → http_get(url)
end
```

A function that calls `exec()` while declaring `forbidden → ["sys.exec"]`
is **rejected at validation time**, before execution.

---

## 2. Capability Tokens

Capability tokens are short-lived, scoped permission grants backed by the
capability manager. They enable fine-grained, revocable access that outlasts
a single function call.

```txtcode
# Acquire a capability token for WiFi scanning
store → tok → grant_capability("wifi.scan", null)

# Use it — valid until explicitly revoked
use_capability(tok)
store → nets → wifi_scan()

# Revoke immediately when done
revoke_capability(tok)
```

Capabilities carry metadata (expiration, AI session info, scope) and are
tracked in the `CapabilityEvent` log. A denied or revoked token raises an
error even if the underlying permission was otherwise granted.

---

## 3. Intent and Forbidden Declarations

Intent declarations document the allowed scope of a function and are enforced
against every privileged call inside that function:

```txtcode
define → port_scan → (host: string) → array
  intent → "network reachability probe only"
  forbidden → ["fs.write", "sys.exec"]
  allowed → ["net.connect"]

  store → results → []
  for → port in [22, 80, 443, 8080]
    store → r → tcp_connect(f"{host}:{port}")
    results += [{"port": port, "open": is_ok(r)}]
  end
  return → results
end
```

If the function body attempts an operation outside its declared intent
(e.g. calling `write_file`), the runtime raises an intent violation and
logs it as `intent.violation.*` in the audit trail.

---

## 4. Audit Trail

Every permission check, capability use, intent violation, and security
startup event is logged to an in-memory audit trail with:

- Monotonic nanosecond timestamp
- Action category (e.g. `fs.read`, `net.connect`, `wifi.scan`)
- Resource (path, hostname, interface, etc.)
- Result: `Allowed` or `Denied`
- AI metadata (model, session, policy version) when present

The audit trail can be exported for post-run review.

---

## 5. Runtime Anti-Debug Protection

The runtime runs a startup security check before executing any user code.

| Check | Platform | Method |
|---|---|---|
| Timing micro-benchmark | All | 50 K-iteration loop; debugger single-step inflates timing above 500 ms threshold |
| TracerPid | Linux | Reads `/proc/self/status`; non-zero TracerPid = ptracer attached |
| wchan | Linux | Reads `/proc/self/wchan`; value `ptrace_stop` = process stepped by ptracer |
| Parent process name | Linux | Reads `/proc/<ppid>/comm`; matches against known debugger list (gdb, lldb, frida, r2, …) |
| Environment injection | All | Checks `LD_PRELOAD`, `LD_AUDIT`, `DYLD_INSERT_LIBRARIES`, Frida markers, suspicious PATH entries |

Findings are classified as `Clean / LowRisk / MediumRisk / HighRisk` and
written to the audit trail under `security.startup`. Execution is not
blocked by default; findings surface as warnings in the audit log.

---

## 6. Script Signing (Ed25519)

Scripts can be signed by the author and verified before execution using
the built-in `ScriptAuth` API. The signature is stored in a sidecar file
(`script.tc.sig`).

```txtcode
# In a setup script — generate a keypair once
store → keys → generate_keypair()   # returns {private_key, public_key}
```

From Rust (e.g. a build tool):

```rust
use txtcode::security::{ScriptAuth, KeyStore};

// Sign
let (priv_key, pub_key) = ScriptAuth::generate_keypair()?;
let sig = ScriptAuth::sign(source_bytes, "author@example.com", &priv_key)?;
std::fs::write("script.tc.sig", sig.to_base64())?;

// Verify before running
let sig = ScriptSignature::from_base64(&std::fs::read_to_string("script.tc.sig")?)?;
assert!(ScriptAuth::verify(source_bytes, &sig)?, "Tampered script!");
```

Signatures cover `SHA-256(content) || signer_id || timestamp` and are
stored as base64 text for easy sidecar distribution.

---

## 7. Bytecode Encryption (AES-256-GCM)

Compiled `.txtc` bytecode can be encrypted before distribution using the
`BytecodeEncryptor`. The key is never derived automatically — you supply it.

```rust
use txtcode::security::{BytecodeEncryptor, EncryptedBytecode};

// Passphrase-derived key (100,000 PBKDF2-HMAC-SHA256 rounds)
let salt = BytecodeEncryptor::generate_salt();
let enc = BytecodeEncryptor::from_passphrase("secret", &salt);

let encrypted = enc.encrypt(&bytecode_bytes)?;
let payload = encrypted.serialize();  // write this to disk
```

> **Important:** Bytecode is **not** encrypted by default. Encryption is
> opt-in and must be applied by the author before distribution.
> The compiled `.txtc` file produced by `txtcode compile` is plain bincode.

---

## 8. Integrity Verification (SHA-256)

`RuntimeSecurity` hashes the source file with SHA-256 before execution
and verifies the hash at startup:

```
hash_and_set_source(source.as_bytes())  # called by run.rs automatically
```

If the source is modified between hash time and execution, the startup
check reports a mismatch in the audit trail under `security.startup`.

---

## 9. What Is NOT Implemented (Stubs)

| Feature | Status | Notes |
|---|---|---|
| AST identifier obfuscation | Stub | `Obfuscator::obfuscate()` returns program unchanged. Planned. |
| macOS anti-debug (kinfo_proc) | Stub | Approach documented in `protector.rs`; needs `libc::sysctl` wiring. |
| Windows anti-debug (IsDebuggerPresent) | Stub | Approach documented in `protector.rs`; needs `winapi` linkage. |

Do not rely on the obfuscator for IP protection — it has no effect in v0.4.

---

## Security Level Summary

On Linux with a source hash provided (normal `txtcode run`), the security
level is **Full**:

```
level=full (timing+os-debugger+integrity)
platform=linux
features=[timing, env-integrity, os-debugger, integrity-capable]
```

On other platforms it is **Basic** (timing + environment scan only) until
the platform-specific anti-debug checks are implemented.
