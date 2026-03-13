// Capability-adaptive runtime security bridge.
//
// DESIGN: Probes available platform features once at startup and dispatches
// each security check to the best implementation available.
//
// ADAPTIVE PRINCIPLE:
//   When `crate::security::protector` gains new OS implementations (e.g., Windows
//   IsDebuggerPresent, macOS kinfo_proc), add the platform to `probe()` and the
//   level upgrades automatically — no other changes required.
//   Same for new VM capabilities (seccomp, cgroups): add to SecurityCapabilities
//   and SecurityLevel, wire once, everything downstream adapts.
//
// SECURITY LEVELS (auto-selected from available capabilities):
//   None     — unknown platform, no probes succeeded
//   Basic    — timing-based detection only (step-through debuggers slow execution)
//   Standard — timing + OS-level debugger detection (Linux: /proc/self/status TracerPid)
//   Full     — timing + OS detection + source file integrity hash verification
//
// FAIL-SECURE SEMANTICS:
//   `true` = secure / no threat found
//   `false` = threat detected
//   When a check cannot run it returns `true` (do not block; no positive evidence of threat).
//   Threats are reported via SecurityCheckReport.warnings and logged to the audit trail.

use crate::security::protector::RuntimeProtector;
use sha2::{Digest, Sha256};

// ── Platform ──────────────────────────────────────────────────────────────────

/// Operating system, detected at compile time via cfg! macros.
#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Linux,
    Windows,
    MacOs,
    Other,
}

impl Platform {
    fn detect() -> Self {
        if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Other
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Platform::Linux => "linux",
            Platform::Windows => "windows",
            Platform::MacOs => "macos",
            Platform::Other => "other",
        };
        write!(f, "{}", s)
    }
}

// ── SecurityCapabilities ──────────────────────────────────────────────────────

/// What security checks can run on the current platform.
///
/// Produced by `SecurityCapabilities::probe()` once at VM startup.
/// Add new fields here when the runtime gains new security primitives.
#[derive(Debug, Clone)]
pub struct SecurityCapabilities {
    pub platform: Platform,
    /// OS-level debugger detection.
    /// Currently real on Linux (/proc/self/status TracerPid).
    /// Set to true for Windows/macOS when protector.rs adds those implementations.
    pub os_debugger_detection: bool,
    /// Timing-based slowdown detection — always true (SystemTime always available).
    pub timing_detection: bool,
    /// Source integrity hash verification — always true (sha2 always compiled in).
    /// Verification activates only when caller calls `set_source_hash()`.
    pub integrity_verification: bool,
    /// Environment injection detection (LD_PRELOAD, Frida markers, etc.) — always available.
    pub env_integrity_detection: bool,
}

impl SecurityCapabilities {
    /// Probe what is actually available and return the capability set.
    ///
    /// This is the single integration point for new platform support.
    /// When protector.rs adds a real Windows or macOS implementation,
    /// flip the corresponding arm here and SecurityLevel auto-upgrades.
    pub fn probe() -> Self {
        let platform = Platform::detect();

        // OS-level debugger detection: now uses 3 techniques on Linux
        // (TracerPid + wchan + parent-process-name).
        // Update Windows/macOS arms when protector.rs adds those implementations.
        let os_debugger_detection = match &platform {
            Platform::Linux => std::fs::read_to_string("/proc/self/status").is_ok(),
            Platform::Windows | Platform::MacOs | Platform::Other => false,
        };

        SecurityCapabilities {
            platform,
            os_debugger_detection,
            timing_detection: true,
            integrity_verification: true,
            env_integrity_detection: true, // env var scanning works on all platforms
        }
    }

    /// Short description for log output.
    pub fn summary(&self) -> String {
        let mut features = vec!["timing", "env-integrity"];
        if self.os_debugger_detection {
            features.push("os-debugger");
        }
        features.push("integrity-capable");
        format!("platform={} features=[{}]", self.platform, features.join(","))
    }
}

// ── SecurityLevel ─────────────────────────────────────────────────────────────

/// Enforcement tier actually running, derived from available capabilities.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SecurityLevel {
    /// No checks active.
    None,
    /// Timing-based detection only.
    Basic,
    /// Timing + OS-level debugger detection.
    Standard,
    /// Timing + OS detection + source integrity verification (hash was provided).
    Full,
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SecurityLevel::None => "none",
            SecurityLevel::Basic => "basic (timing)",
            SecurityLevel::Standard => "standard (timing+os-debugger)",
            SecurityLevel::Full => "full (timing+os-debugger+integrity)",
        };
        write!(f, "{}", s)
    }
}

// ── SecurityCheckReport ───────────────────────────────────────────────────────

/// Structured results from a `run_startup_checks()` call.
/// Logged to the audit trail on every `interpret()` invocation.
#[derive(Debug, Clone)]
pub struct SecurityCheckReport {
    pub level: SecurityLevel,
    /// `Some(true)` = clean; `Some(false)` = threat detected; `None` = not checked.
    pub anti_debug: Option<bool>,
    /// `Some(true)` = hash matched; `Some(false)` = mismatch; `None` = no hash provided.
    pub integrity: Option<bool>,
    /// `Some(true)` = environment clean; `Some(false)` = injection/hooking detected.
    pub environment: Option<bool>,
    pub platform: Platform,
    /// Non-empty when a threat or misconfiguration was detected.
    pub warnings: Vec<String>,
}

impl SecurityCheckReport {
    /// True if all performed checks passed (unchecked fields are not threats).
    pub fn is_secure(&self) -> bool {
        self.anti_debug.unwrap_or(true)
            && self.integrity.unwrap_or(true)
            && self.environment.unwrap_or(true)
    }

    /// One-line summary suitable for audit trail entries.
    pub fn summary(&self) -> String {
        let ad = match self.anti_debug {
            Some(true) => "clean",
            Some(false) => "THREAT",
            None => "unchecked",
        };
        let ig = match self.integrity {
            Some(true) => "ok",
            Some(false) => "MISMATCH",
            None => "unchecked",
        };
        let env = match self.environment {
            Some(true) => "clean",
            Some(false) => "SUSPICIOUS",
            None => "unchecked",
        };
        let w = if self.warnings.is_empty() {
            String::new()
        } else {
            format!(" warnings=[{}]", self.warnings.join("; "))
        };
        format!(
            "level={} anti_debug={} integrity={} env={} platform={}{}",
            self.level, ad, ig, env, self.platform, w
        )
    }
}

// ── RuntimeSecurity ───────────────────────────────────────────────────────────

/// Capability-adaptive runtime security manager.
///
/// Replaces the old no-op stub. Wraps `RuntimeProtector` and dispatches each
/// check to the best implementation for the current platform. Integrates with
/// the VM audit trail via `SecurityCheckReport`.
///
/// Usage in VM:
///   1. `VirtualMachine::new()` creates `RuntimeSecurity::new()` (probes capabilities).
///   2. `run.rs` calls `vm.runtime_security.hash_and_set_source(source.as_bytes())`
///      to activate integrity checking before `interpret()`.
///   3. `interpret()` calls `run_startup_checks()` and logs the report to audit trail.
pub struct RuntimeSecurity {
    capabilities: SecurityCapabilities,
    protector: RuntimeProtector,
    /// Our copy of the source hash (needed to pass to protector.verify_integrity).
    source_hash: Option<Vec<u8>>,
}

impl RuntimeSecurity {
    /// Create with auto-probed capabilities.
    pub fn new() -> Self {
        let capabilities = SecurityCapabilities::probe();
        let protector = RuntimeProtector::new();
        Self {
            capabilities,
            protector,
            source_hash: None,
        }
    }

    // ── Source hash ───────────────────────────────────────────────────────

    /// Set an already-computed SHA-256 hash of the source file.
    /// Activates integrity verification and upgrades SecurityLevel toward Full.
    pub fn set_source_hash(&mut self, hash: Vec<u8>) {
        self.protector = RuntimeProtector::with_integrity_hash(hash.clone());
        self.source_hash = Some(hash);
    }

    /// Hash raw source bytes with SHA-256 and store the result.
    /// Convenience method for `run.rs` before calling `vm.interpret()`.
    pub fn hash_and_set_source(&mut self, source_bytes: &[u8]) {
        let mut hasher = Sha256::new();
        hasher.update(source_bytes);
        self.set_source_hash(hasher.finalize().to_vec());
    }

    // ── Level and capabilities ────────────────────────────────────────────

    /// Current enforcement level, auto-derived from available capabilities.
    ///
    /// Automatically upgrades when:
    ///   - `os_debugger_detection` becomes true (new OS implementation in protector.rs)
    ///   - `source_hash` is provided (call `set_source_hash` or `hash_and_set_source`)
    pub fn level(&self) -> SecurityLevel {
        match (
            self.capabilities.timing_detection,
            self.capabilities.os_debugger_detection,
            self.source_hash.is_some(),
        ) {
            (true, true, true) => SecurityLevel::Full,
            (true, true, false) => SecurityLevel::Standard,
            (true, false, _) => SecurityLevel::Basic,
            _ => SecurityLevel::None,
        }
    }

    /// Read-only access to the probed capabilities.
    pub fn capabilities(&self) -> &SecurityCapabilities {
        &self.capabilities
    }

    // ── Checks ────────────────────────────────────────────────────────────

    /// Run all available checks and return a structured report.
    ///
    /// Called once at the start of `interpret()`. The report is logged to the
    /// audit trail. Execution is NOT blocked — this is a detect-and-report layer.
    /// For a pentesting tool, the operator should inspect warnings in the audit trail.
    pub fn run_startup_checks(&self) -> SecurityCheckReport {
        let mut warnings: Vec<String> = Vec::new();
        let mut anti_debug: Option<bool> = None;
        let mut integrity: Option<bool> = None;
        let mut environment: Option<bool> = None;

        // ── Anti-debugging ────────────────────────────────────────────────
        // Protector now uses: timing (all platforms) + TracerPid + wchan +
        // parent-process-name (Linux). Returns true = safe, false = threat.
        if self.capabilities.timing_detection || self.capabilities.os_debugger_detection {
            let safe = self.protector.check_anti_debugging();
            anti_debug = Some(safe);
            if !safe {
                let detail = if self.capabilities.os_debugger_detection {
                    format!(
                        "Debugger detected ({}): TracerPid, wchan, or parent process check triggered.",
                        self.capabilities.platform
                    )
                } else {
                    "Abnormal execution timing: possible step-through debugging or severe load."
                        .to_string()
                };
                warnings.push(detail);
            }
        }

        // ── Environment integrity ─────────────────────────────────────────
        // Check LD_PRELOAD, LD_AUDIT, Frida markers, etc. (all platforms).
        if self.capabilities.env_integrity_detection {
            let env_result = self.protector.check_environment_integrity();
            let clean = env_result.is_clean();
            environment = Some(clean);
            if !clean {
                for finding in &env_result.findings {
                    warnings.push(format!(
                        "Environment integrity [{:?}]: {}",
                        env_result.risk, finding
                    ));
                }
            }
        }

        // ── Source integrity ──────────────────────────────────────────────
        if let Some(hash) = &self.source_hash {
            let ok = self.protector.verify_integrity(hash);
            integrity = Some(ok);
            if !ok {
                warnings.push(
                    "Source integrity check failed: hash mismatch. \
                     Runtime state may have been tampered with."
                        .to_string(),
                );
            }
        }

        // ── Level advisory ────────────────────────────────────────────────
        if self.level() < SecurityLevel::Standard {
            warnings.push(format!(
                "Security level is {} (platform={}). \
                 OS-level debugger detection not available on this platform.",
                self.level(),
                self.capabilities.platform
            ));
        }

        SecurityCheckReport {
            level: self.level(),
            anti_debug,
            integrity,
            environment,
            platform: self.capabilities.platform.clone(),
            warnings,
        }
    }

    /// On-demand anti-debugging check.
    /// Returns `true` (safe) if no threat is detected, or if no check is available.
    pub fn check_anti_debugging(&self) -> bool {
        if self.capabilities.timing_detection || self.capabilities.os_debugger_detection {
            self.protector.check_anti_debugging()
        } else {
            true // No check available; assume safe — do not block execution.
        }
    }

    /// On-demand integrity verification.
    /// Returns `true` if hash matches or if no hash was provided.
    pub fn verify_integrity(&self) -> bool {
        if let Some(hash) = &self.source_hash {
            self.protector.verify_integrity(hash)
        } else {
            true // No hash registered; skip check (not a failure).
        }
    }
}

impl Default for RuntimeSecurity {
    fn default() -> Self {
        Self::new()
    }
}
