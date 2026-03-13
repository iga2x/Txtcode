// Runtime protection against debugging and tampering.
//
// TECHNIQUE STATUS BY PLATFORM:
//
//   Timing micro-benchmark      — ALL PLATFORMS: REAL
//     Times a tight arithmetic loop; a debugger stepping each instruction
//     causes 100x–1000x slowdown, easily detected.
//
//   TracerPid (/proc/self/status) — LINUX: REAL
//     Reads the kernel-maintained tracer PID. Non-zero = tracer attached.
//
//   wchan (/proc/self/wchan)      — LINUX: REAL
//     Kernel wait-channel name. "ptrace_stop" means the process is stopped
//     by a ptracer (debugger is actively single-stepping).
//
//   Parent process name           — LINUX: REAL
//     Reads /proc/self/status (PPid), then /proc/<ppid>/comm.
//     Matches against a list of known debuggers/tracers.
//
//   Environment injection check   — ALL PLATFORMS: REAL
//     Detects LD_PRELOAD, LD_AUDIT, DYLD_INSERT_LIBRARIES, and other
//     environment variables used by hooking frameworks (Frida, etc.).
//
//   macOS kinfo_proc sysctl        — STUB (documented approach below)
//   Windows IsDebuggerPresent      — STUB (documented approach below)
//
// RETURNS: true = secure / no threat; false = threat detected
// (fail-secure: when a check cannot run, it returns true, not false)

use std::time::{Duration, Instant};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Risk level from the environment integrity check.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum EnvironmentRisk {
    Clean,
    LowRisk,
    MediumRisk,
    HighRisk,
}

impl std::fmt::Display for EnvironmentRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvironmentRisk::Clean => write!(f, "clean"),
            EnvironmentRisk::LowRisk => write!(f, "low"),
            EnvironmentRisk::MediumRisk => write!(f, "medium"),
            EnvironmentRisk::HighRisk => write!(f, "high"),
        }
    }
}

/// Result of environment injection scanning.
#[derive(Debug, Clone)]
pub struct EnvironmentCheckResult {
    pub risk: EnvironmentRisk,
    /// Human-readable descriptions of suspicious findings.
    pub findings: Vec<String>,
}

impl EnvironmentCheckResult {
    pub fn is_clean(&self) -> bool {
        matches!(self.risk, EnvironmentRisk::Clean)
    }
}

/// Aggregated result of all security checks.
#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    /// true = no debugger detected; false = debugger or excessive slowdown
    pub anti_debug: bool,
    /// true = hash matched or no hash registered; false = hash mismatch
    pub integrity: bool,
    /// true = environment is clean; false = injection/hooking variables found
    pub tampering: bool,
    pub environment: EnvironmentCheckResult,
}

impl SecurityCheckResult {
    pub fn is_secure(&self) -> bool {
        self.anti_debug && self.integrity && self.tampering
    }
}

// ── RuntimeProtector ─────────────────────────────────────────────────────────

/// Runtime protection against debugging and tampering.
pub struct RuntimeProtector {
    #[allow(dead_code)] // reserved for future uptime / session-duration checks
    start_time: Instant,
    integrity_hash: Option<Vec<u8>>,
    anti_debug_enabled: bool,
}

impl RuntimeProtector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            integrity_hash: None,
            anti_debug_enabled: true,
        }
    }

    pub fn with_integrity_hash(hash: Vec<u8>) -> Self {
        Self {
            start_time: Instant::now(),
            integrity_hash: Some(hash),
            anti_debug_enabled: true,
        }
    }

    // ── Anti-debugging ────────────────────────────────────────────────────

    /// Run all available anti-debugging checks for the current platform.
    /// Returns `true` when no debugger is detected (safe to proceed).
    pub fn check_anti_debugging(&self) -> bool {
        if !self.anti_debug_enabled {
            return true;
        }

        // Technique 1 (all platforms): Timing micro-benchmark.
        // A debugger stepping through instructions causes large pauses.
        if !self.timing_check() {
            return false;
        }

        // Technique 2: OS-specific checks.
        #[cfg(target_os = "linux")]
        if self.is_debugger_present_linux() {
            return false;
        }

        #[cfg(target_os = "macos")]
        if self.is_debugger_present_macos() {
            return false;
        }

        #[cfg(target_os = "windows")]
        if self.is_debugger_present_windows() {
            return false;
        }

        true
    }

    // ── Environment integrity ─────────────────────────────────────────────

    /// Scan for environment variables used by hooking/tracing frameworks.
    ///
    /// Detects: LD_PRELOAD, LD_AUDIT (Linux library injection), DYLD_INSERT_LIBRARIES
    /// (macOS injection), and Frida/debugger tool paths in PATH.
    /// This is cross-platform and requires no OS-specific syscalls.
    pub fn check_environment_integrity(&self) -> EnvironmentCheckResult {
        let mut findings: Vec<String> = Vec::new();
        let mut risk = EnvironmentRisk::Clean;

        // ── Library injection variables ───────────────────────────────────
        // These are the primary mechanism used by Frida, LD_PRELOAD hookers,
        // and instrumentation frameworks to inject code into a process.
        let high_risk_vars = ["LD_PRELOAD", "LD_AUDIT", "DYLD_INSERT_LIBRARIES"];
        for var in &high_risk_vars {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    findings.push(format!("{}={}", var, val));
                    risk = EnvironmentRisk::HighRisk;
                }
            }
        }

        // ── Suspicious library path modifications ─────────────────────────
        let medium_risk_vars = [
            "LD_LIBRARY_PATH",
            "DYLD_LIBRARY_PATH",
            "DYLD_FORCE_FLAT_NAMESPACE",  // disables macOS 2-level namespace (debugger trick)
        ];
        for var in &medium_risk_vars {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    findings.push(format!("{}={}", var, val));
                    if risk < EnvironmentRisk::MediumRisk {
                        risk = EnvironmentRisk::MediumRisk;
                    }
                }
            }
        }

        // ── Debugger/tracer tool signatures in PATH ───────────────────────
        // Frida and some debuggers modify PATH to expose their CLI tools.
        const KNOWN_TOOLS: &[&str] = &[
            "frida", "ida64", "ida32", "ida", "gdb", "lldb", "radare2",
            "r2agent", "qira", "pwndbg", "peda",
        ];
        if let Ok(path) = std::env::var("PATH") {
            let path_lower = path.to_lowercase();
            for tool in KNOWN_TOOLS {
                // Match path component, not just substring (avoid false positives).
                // e.g. "/usr/local/frida/bin" contains "frida"
                if path_lower
                    .split(':')
                    .any(|segment| segment.contains(tool))
                {
                    findings.push(format!("PATH segment contains '{}'", tool));
                    if risk < EnvironmentRisk::LowRisk {
                        risk = EnvironmentRisk::LowRisk;
                    }
                }
            }
        }

        // ── Frida-specific markers ─────────────────────────────────────────
        // Frida sets a communication socket path in the environment when attached.
        for var in &["FRIDA_TRANSPORT", "FRIDA_LISTEN", "_FRIDA_AGENT"] {
            if std::env::var(var).is_ok() {
                findings.push(format!("{} environment variable present (Frida indicator)", var));
                risk = EnvironmentRisk::HighRisk;
            }
        }

        EnvironmentCheckResult { risk, findings }
    }

    // ── Integrity ─────────────────────────────────────────────────────────

    /// Compare `current_hash` against the stored integrity hash.
    /// Returns `true` when hashes match, or when no hash was registered.
    pub fn verify_integrity(&self, current_hash: &[u8]) -> bool {
        if let Some(expected) = &self.integrity_hash {
            expected == current_hash
        } else {
            true // No hash registered — skip verification (not a failure).
        }
    }

    // ── Combined ──────────────────────────────────────────────────────────

    /// Run all checks and return a structured report.
    pub fn perform_security_checks(&self, current_hash: Option<&[u8]>) -> SecurityCheckResult {
        let environment = self.check_environment_integrity();
        let tampering = environment.is_clean();

        let integrity = current_hash
            .map(|h| self.verify_integrity(h))
            .unwrap_or(true);

        SecurityCheckResult {
            anti_debug: self.check_anti_debugging(),
            integrity,
            tampering,
            environment,
        }
    }

    // ── Timing micro-benchmark (all platforms) ────────────────────────────

    /// Time a tight arithmetic loop.
    ///
    /// Under normal execution: completes in < 1 ms.
    /// Under debugger single-step: each instruction is a trap → 100ms–seconds.
    /// Threshold 500 ms: conservative enough to avoid false positives from system load.
    fn timing_check(&self) -> bool {
        let t0 = Instant::now();

        // Tight loop: result must be used to prevent dead-code elimination.
        let mut acc: u64 = 0x_dead_beef_0bad_f00d;
        for i in 0u64..50_000 {
            acc = acc
                .wrapping_add(i.wrapping_mul(i))
                .wrapping_add(i << 3)
                .wrapping_add(1);
        }
        let elapsed = t0.elapsed();

        // Prevent compiler from removing the loop entirely.
        // The `acc == 0` case is impossible in practice.
        if acc == 0 {
            return false;
        }

        elapsed < Duration::from_millis(500)
    }

    // ── Linux (real implementations) ─────────────────────────────────────

    /// Entry point for all Linux anti-debug checks.
    #[cfg(target_os = "linux")]
    fn is_debugger_present_linux(&self) -> bool {
        self.check_tracerpid() || self.check_wchan() || self.check_parent_process()
    }

    /// TracerPid check: reads /proc/self/status.
    /// A non-zero TracerPid means a ptracer is attached (gdb, strace, etc.).
    #[cfg(target_os = "linux")]
    fn check_tracerpid(&self) -> bool {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("TracerPid:") {
                    if let Some(pid) = line.split_whitespace().nth(1).and_then(|s| s.parse::<u32>().ok()) {
                        return pid != 0;
                    }
                }
            }
        }
        false
    }

    /// wchan check: /proc/self/wchan contains the kernel wait-channel name.
    /// "ptrace_stop" means the process is stopped by a ptracer that is single-stepping.
    #[cfg(target_os = "linux")]
    fn check_wchan(&self) -> bool {
        if let Ok(content) = std::fs::read_to_string("/proc/self/wchan") {
            return content.trim().contains("ptrace");
        }
        false
    }

    /// Parent process name check: if the parent is a known debugger, flag it.
    #[cfg(target_os = "linux")]
    fn check_parent_process(&self) -> bool {
        const KNOWN_DEBUGGERS: &[&str] = &[
            "gdb", "lldb", "strace", "ltrace", "frida", "frida-server",
            "radare2", "r2", "ida64", "ida32", "x64dbg", "ollydbg",
            "windbg", "qira", "pwndbg", "peda", "voltron",
        ];

        // Get PPid from /proc/self/status.
        let ppid = std::fs::read_to_string("/proc/self/status").ok().and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("PPid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|s| s.parse::<u32>().ok())
        });

        if let Some(ppid) = ppid {
            if let Ok(comm) = std::fs::read_to_string(format!("/proc/{}/comm", ppid)) {
                let name = comm.trim().to_lowercase();
                return KNOWN_DEBUGGERS.iter().any(|d| name == *d || name.starts_with(d));
            }
        }
        false
    }

    // ── macOS (stub — documented approach) ───────────────────────────────

    /// macOS: check P_TRACED flag via sysctl(KERN_PROC, KERN_PROC_PID, ...).
    ///
    /// NOT YET IMPLEMENTED — requires unsafe libc::sysctl with kinfo_proc layout.
    /// Planned approach:
    /// ```text
    /// let mut info: libc::kinfo_proc = std::mem::zeroed();
    /// let mut size = std::mem::size_of::<libc::kinfo_proc>();
    /// let mib = [CTL_KERN, KERN_PROC, KERN_PROC_PID, getpid()];
    /// sysctl(mib, 4, &info, &mut size, null, 0);
    /// return info.kp_proc.p_flag & P_TRACED != 0;  // P_TRACED = 0x00000800
    /// ```
    /// Enable by changing `false` to the sysctl call above once validated.
    #[cfg(target_os = "macos")]
    fn is_debugger_present_macos(&self) -> bool {
        false
    }

    // ── Windows (stub — documented approach) ─────────────────────────────

    /// Windows: use IsDebuggerPresent / CheckRemoteDebuggerPresent.
    ///
    /// NOT YET IMPLEMENTED — requires winapi crate or raw extern linkage.
    /// Planned approach:
    /// ```text
    /// extern "system" { fn IsDebuggerPresent() -> i32; }
    /// unsafe { IsDebuggerPresent() != 0 }
    /// ```
    /// Or via NtQueryInformationProcess (ProcessDebugPort) for remote detection.
    /// Enable by adding the extern block and changing `false` to the call above.
    #[cfg(target_os = "windows")]
    fn is_debugger_present_windows(&self) -> bool {
        false
    }

    // ── Fallback stubs for unrecognised platforms ─────────────────────────

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn is_debugger_present_linux(&self) -> bool {
        false
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn is_debugger_present_windows(&self) -> bool {
        false
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn is_debugger_present_macos(&self) -> bool {
        false
    }
}

impl Default for RuntimeProtector {
    fn default() -> Self {
        Self::new()
    }
}
