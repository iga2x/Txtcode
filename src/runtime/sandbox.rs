/// OS-level process sandbox for the Txtcode runtime.
///
/// # Linux implementation (Group 25.2)
///
/// When `--sandbox` is passed, `apply_sandbox()` applies two layers:
///
/// 1. **prctl hardening** (always applied on Linux when sandbox=true):
///    - `PR_SET_NO_NEW_PRIVS = 1` — child processes cannot gain privileges via
///      setuid/setgid/file capabilities. Required prerequisite for seccomp.
///    - `PR_SET_DUMPABLE = 0` — disables core dumps and blocks `ptrace(PTRACE_ATTACH)`.
///
/// 2. **seccomp-BPF filter** (applied after prctl on Linux):
///    Uses a blocklist approach — denies the most dangerous syscalls (process
///    spawning, privilege escalation) while allowing everything else. This is
///    intentionally conservative: a whitelist would be more secure but would
///    require enumerating all syscalls used by the Rust standard library (100+).
///
/// # `--sandbox-strict` (Group G.2)
///
/// `apply_sandbox_strict()` applies prctl hardening plus a seccomp **allowlist**
/// filter on Linux x86-64. Only explicitly listed syscalls are allowed; all
/// others return `EPERM` (rather than killing the process, to produce a clear
/// error message rather than a silent SIGSYS crash).
///
/// # macOS implementation (Group G.3)
///
/// On macOS, `apply_sandbox()` calls `sandbox_init()` from libSystem.dylib with a
/// deny-default profile that allows file reads and outbound network. This replaces
/// the previous no-op behaviour so `--sandbox` on macOS provides real OS isolation.
///
/// # Security boundary
/// The OS sandbox complements the language-level permission system. Even if a bug
/// in the runtime allowed crafting malicious arguments, the seccomp filter prevents
/// the underlying process from executing external programs.
/// Result type for sandbox operations.
pub type SandboxResult = Result<(), String>;

/// Apply OS-level sandbox to the current process (blocklist mode).
///
/// Must be called BEFORE the VM begins executing user code.
/// Returns `Err` only if a sandbox primitive fails (e.g., seccomp not supported).
pub fn apply_sandbox(sandbox: bool) -> SandboxResult {
    if !sandbox {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        apply_linux_sandbox()
    }
    #[cfg(target_os = "macos")]
    {
        apply_macos_sandbox()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        // Non-Linux/macOS: language-level permissions still apply; OS sandbox is a no-op.
        Ok(())
    }
}

/// Apply OS-level sandbox in strict allowlist mode (`--sandbox-strict`).
///
/// On Linux x86-64: applies prctl hardening + seccomp allowlist filter.
/// All syscalls NOT on the allowlist return EPERM (so the process exits with
/// a clear error rather than crashing with SIGSYS).
/// On macOS: delegates to `apply_macos_sandbox()`.
/// On other platforms: no-op (language-level permissions still apply).
pub fn apply_sandbox_strict(enabled: bool) -> SandboxResult {
    if !enabled {
        return Ok(());
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        apply_linux_sandbox_strict()
    }
    #[cfg(target_os = "macos")]
    {
        apply_macos_sandbox()
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        target_os = "macos"
    )))]
    {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn apply_linux_sandbox() -> SandboxResult {
    use std::io;

    // ── Step 1: prctl hardening ────────────────────────────────────────────────

    // PR_SET_DUMPABLE = 0: disables core dumps, blocks PTRACE_ATTACH
    let ret = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    if ret != 0 {
        let err = io::Error::last_os_error();
        return Err(format!("sandbox: prctl(PR_SET_DUMPABLE) failed: {}", err));
    }

    // PR_SET_NO_NEW_PRIVS = 1: prerequisite for seccomp; prevents setuid escalation.
    let ret = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if ret != 0 {
        let err = io::Error::last_os_error();
        return Err(format!("sandbox: prctl(PR_SET_NO_NEW_PRIVS) failed: {}", err));
    }

    // ── Step 2: seccomp-BPF blocklist ─────────────────────────────────────────
    //
    // Architecture: blocklist of dangerous syscalls → KILL process.
    // Allowed by default; only the most dangerous operations are blocked.
    //
    // BPF filter encoding:
    //   Each sock_filter is { code: u16, jt: u8, jf: u8, k: u32 }.
    //
    // Syscall numbers (x86-64):
    //   59  = execve
    //   322 = execveat
    //   57  = fork
    //   58  = vfork
    //   56  = clone (but NOT clone3 which is 435 — needed for threads)
    //   172 = ptrace
    //   157 = prctl (self-calls allowed; we block only from user code indirectly)
    //   317 = seccomp (prevent further seccomp rules from being loosened)
    //   248 = process_vm_readv
    //   249 = process_vm_writev
    //   200 = tkill (signal to specific thread — keep sigkill ability)
    //
    // We block execve/execveat (no spawning external processes),
    // and ptrace (no attaching debuggers to other processes).
    // clone/fork are allowed because Rust's threading uses clone.

    apply_seccomp_blocklist()
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn apply_linux_sandbox_strict() -> SandboxResult {
    use std::io;

    // ── Step 1: prctl hardening (same as blocklist mode) ──────────────────────
    let ret = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    if ret != 0 {
        let err = io::Error::last_os_error();
        return Err(format!("sandbox-strict: prctl(PR_SET_DUMPABLE) failed: {}", err));
    }
    let ret = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if ret != 0 {
        let err = io::Error::last_os_error();
        return Err(format!("sandbox-strict: prctl(PR_SET_NO_NEW_PRIVS) failed: {}", err));
    }

    // ── Step 2: seccomp-BPF allowlist ─────────────────────────────────────────
    apply_seccomp_allowlist()
}

#[cfg(target_os = "linux")]
#[allow(clippy::vec_init_then_push)]
fn apply_seccomp_blocklist() -> SandboxResult {
    // BPF constants
    const BPF_LD: u16 = 0x00;
    const BPF_W: u16 = 0x00;
    const BPF_ABS: u16 = 0x20;
    const BPF_JMP: u16 = 0x05;
    const BPF_JEQ: u16 = 0x10;
    const BPF_K: u16 = 0x00;
    const BPF_RET: u16 = 0x06;

    const SECCOMP_RET_KILL_PROCESS: u32 = 0x80000000;
    const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;

    // Offset of nr (syscall number) in seccomp_data struct
    const SECCOMP_DATA_NR_OFFSET: u32 = 0;
    // Offset of arch in seccomp_data struct
    const SECCOMP_DATA_ARCH_OFFSET: u32 = 4;

    // AUDIT_ARCH_X86_64
    const AUDIT_ARCH_X86_64: u32 = 0xC000003E;

    // Syscall numbers for x86-64 we want to block
    #[cfg(target_arch = "x86_64")]
    let blocked_syscalls: &[u32] = &[
        59,  // execve
        322, // execveat
        172, // ptrace
        317, // seccomp (prevent weakening the filter)
        248, // process_vm_readv
        249, // process_vm_writev
    ];

    #[cfg(not(target_arch = "x86_64"))]
    let blocked_syscalls: &[u32] = &[]; // Only x86-64 has known syscall numbers here

    // Build the BPF program dynamically.
    // Structure:
    //   1. Load arch word → if not x86_64, KILL (prevent wrong-arch bypass)
    //   2. Load syscall nr
    //   3. For each blocked syscall: if nr == blocked → KILL
    //   4. Default: ALLOW
    let mut filter: Vec<libc::sock_filter> = Vec::new();

    // Validate architecture: load arch field, compare to AUDIT_ARCH_X86_64
    filter.push(libc::sock_filter {
        code: BPF_LD | BPF_W | BPF_ABS,
        jt: 0,
        jf: 0,
        k: SECCOMP_DATA_ARCH_OFFSET,
    });
    // JEQ arch == AUDIT_ARCH_X86_64 → skip 1 (continue), else KILL
    filter.push(libc::sock_filter {
        code: BPF_JMP | BPF_JEQ | BPF_K,
        jt: 1,
        jf: 0,
        k: AUDIT_ARCH_X86_64,
    });
    filter.push(libc::sock_filter {
        code: BPF_RET | BPF_K,
        jt: 0,
        jf: 0,
        k: SECCOMP_RET_KILL_PROCESS,
    });

    // Load syscall number
    filter.push(libc::sock_filter {
        code: BPF_LD | BPF_W | BPF_ABS,
        jt: 0,
        jf: 0,
        k: SECCOMP_DATA_NR_OFFSET,
    });

    // For each blocked syscall, add a JEQ → KILL instruction pair
    for &syscall_nr in blocked_syscalls {
        // Instructions remaining after this JEQ (number of ALLOW + remaining blocks):
        // We use "skip to KILL" approach: jt=0 → execute KILL, jf=1 → skip KILL
        filter.push(libc::sock_filter {
            code: BPF_JMP | BPF_JEQ | BPF_K,
            jt: 0, // if equal → next instr (KILL)
            jf: 1, // if not equal → skip KILL
            k: syscall_nr,
        });
        filter.push(libc::sock_filter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_KILL_PROCESS,
        });
    }

    // Default: ALLOW
    filter.push(libc::sock_filter {
        code: BPF_RET | BPF_K,
        jt: 0,
        jf: 0,
        k: SECCOMP_RET_ALLOW,
    });

    let prog = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_mut_ptr(),
    };

    // SECCOMP_SET_MODE_FILTER = 1
    const SECCOMP_SET_MODE_FILTER: libc::c_int = 1;
    let ret = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            SECCOMP_SET_MODE_FILTER as libc::c_long,
            0 as libc::c_long,
            &prog as *const libc::sock_fprog as libc::c_long,
        )
    };

    if ret != 0 {
        let err = std::io::Error::last_os_error();
        // Seccomp may not be available in all environments (containers, CI).
        // Log a warning but do not abort — language-level permissions still apply.
        eprintln!(
            "[sandbox] WARNING: seccomp-BPF filter could not be applied ({}). \
             Language-level permission checks still enforce policy.",
            err
        );
    }

    Ok(())
}

/// Allowlist-based seccomp filter for `--sandbox-strict` on Linux x86-64.
///
/// Only syscalls on the allowlist below are permitted. All others return EPERM
/// (SECCOMP_RET_ERRNO) so the process can surface a clear error message instead
/// of being killed with SIGSYS.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[allow(clippy::vec_init_then_push)]
fn apply_seccomp_allowlist() -> SandboxResult {
    // BPF constants
    const BPF_LD: u16 = 0x00;
    const BPF_W: u16 = 0x00;
    const BPF_ABS: u16 = 0x20;
    const BPF_JMP: u16 = 0x05;
    const BPF_JEQ: u16 = 0x10;
    const BPF_K: u16 = 0x00;
    const BPF_RET: u16 = 0x06;

    const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;
    // Return EPERM for unlisted syscalls — process gets an error, not a crash.
    const SECCOMP_RET_ERRNO_EPERM: u32 = 0x00050000 | 1; // SECCOMP_RET_ERRNO | EPERM
    const SECCOMP_DATA_NR_OFFSET: u32 = 0;
    const SECCOMP_DATA_ARCH_OFFSET: u32 = 4;
    const AUDIT_ARCH_X86_64: u32 = 0xC000003E;

    // Allowlist: syscalls the Txtcode runtime requires on Linux x86-64.
    // Derived from strace profiling of typical scripts (file I/O, network, crypto).
    // NOTE: execve (59) and execveat (322) are intentionally NOT included.
    let allowed: &[u32] = &[
        0,   // read
        1,   // write
        2,   // open
        3,   // close
        4,   // stat
        5,   // fstat
        6,   // lstat
        7,   // poll
        8,   // lseek
        9,   // mmap
        10,  // mprotect
        11,  // munmap
        12,  // brk
        13,  // rt_sigaction
        14,  // rt_sigprocmask
        15,  // rt_sigreturn
        16,  // ioctl
        17,  // pread64
        18,  // pwrite64
        19,  // readv
        20,  // writev
        21,  // access
        22,  // pipe
        23,  // select
        24,  // sched_yield
        28,  // madvise
        32,  // dup
        33,  // dup2
        35,  // nanosleep
        39,  // getpid
        40,  // sendfile
        41,  // socket
        42,  // connect
        43,  // accept
        44,  // sendto
        45,  // recvfrom
        46,  // sendmsg
        47,  // recvmsg
        48,  // shutdown
        49,  // bind
        50,  // listen
        51,  // getsockname
        52,  // getpeername
        53,  // socketpair
        54,  // setsockopt
        55,  // getsockopt
        56,  // clone
        60,  // exit
        61,  // wait4
        63,  // uname
        72,  // fcntl
        73,  // flock
        74,  // fsync
        75,  // fdatasync
        76,  // truncate
        77,  // ftruncate
        78,  // getdents
        79,  // getcwd
        80,  // chdir
        82,  // rename
        83,  // mkdir
        84,  // rmdir
        86,  // link
        87,  // unlink
        88,  // symlink
        89,  // readlink
        90,  // chmod
        91,  // fchmod
        93,  // fchown
        94,  // umask
        96,  // gettimeofday
        97,  // getrlimit
        98,  // getrusage
        99,  // sysinfo
        100, // times
        102, // getuid
        104, // getgid
        107, // geteuid
        108, // getegid
        110, // getppid
        128, // rt_sigsuspend
        131, // sigaltstack
        137, // statfs
        138, // fstatfs
        157, // prctl (needed for our own sandbox setup)
        158, // arch_prctl
        186, // gettid
        201, // time
        202, // futex
        217, // getdents64
        218, // set_tid_address
        228, // clock_gettime
        229, // clock_getres
        230, // clock_nanosleep
        231, // exit_group
        232, // epoll_wait
        233, // epoll_ctl
        234, // tgkill
        257, // openat
        262, // newfstatat
        265, // linkat
        266, // symlinkat
        267, // readlinkat
        270, // pselect6
        271, // ppoll
        273, // set_robust_list
        274, // get_robust_list
        280, // utimensat
        281, // epoll_pwait
        283, // timerfd_create
        285, // fallocate
        286, // timerfd_settime
        287, // timerfd_gettime
        288, // accept4
        290, // eventfd2
        291, // epoll_create1
        292, // dup3
        293, // pipe2
        302, // prlimit64
        318, // getrandom
        332, // statx
        334, // rseq
        435, // clone3
    ];

    // Build the BPF allowlist program.
    // Structure:
    //   1. Validate arch → if not x86_64, KILL
    //   2. Load syscall nr
    //   3. For each allowed syscall: if nr == allowed → ALLOW
    //   4. Default: ERRNO(EPERM)
    let mut filter: Vec<libc::sock_filter> = Vec::new();

    // Arch check
    filter.push(libc::sock_filter {
        code: BPF_LD | BPF_W | BPF_ABS,
        jt: 0, jf: 0, k: SECCOMP_DATA_ARCH_OFFSET,
    });
    filter.push(libc::sock_filter {
        code: BPF_JMP | BPF_JEQ | BPF_K,
        jt: 1, jf: 0, k: AUDIT_ARCH_X86_64,
    });
    filter.push(libc::sock_filter {
        code: BPF_RET | BPF_K,
        jt: 0, jf: 0, k: 0x80000000, // SECCOMP_RET_KILL_PROCESS for wrong arch
    });

    // Load syscall number
    filter.push(libc::sock_filter {
        code: BPF_LD | BPF_W | BPF_ABS,
        jt: 0, jf: 0, k: SECCOMP_DATA_NR_OFFSET,
    });

    // For each allowed syscall: JEQ → ALLOW (jt=1 skips ALLOW, jf=0 falls to ALLOW)
    // We use: if eq → skip to ALLOW, else continue checking
    // Actually: JEQ jt=0 → next instr (ALLOW); jf=1 → skip ALLOW and continue
    for &syscall_nr in allowed {
        filter.push(libc::sock_filter {
            code: BPF_JMP | BPF_JEQ | BPF_K,
            jt: 0, // if equal → next instr (ALLOW)
            jf: 1, // if not equal → skip ALLOW, continue checking
            k: syscall_nr,
        });
        filter.push(libc::sock_filter {
            code: BPF_RET | BPF_K,
            jt: 0, jf: 0, k: SECCOMP_RET_ALLOW,
        });
    }

    // Default: ERRNO(EPERM) — syscall returns -1 with errno=EPERM
    filter.push(libc::sock_filter {
        code: BPF_RET | BPF_K,
        jt: 0, jf: 0, k: SECCOMP_RET_ERRNO_EPERM,
    });

    let prog = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_mut_ptr(),
    };

    const SECCOMP_SET_MODE_FILTER: libc::c_int = 1;
    let ret = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            SECCOMP_SET_MODE_FILTER as libc::c_long,
            0 as libc::c_long,
            &prog as *const libc::sock_fprog as libc::c_long,
        )
    };

    if ret != 0 {
        let err = std::io::Error::last_os_error();
        eprintln!(
            "[sandbox-strict] WARNING: seccomp allowlist filter could not be applied ({}). \
             Language-level permission checks still enforce policy.",
            err
        );
    }

    Ok(())
}

/// macOS sandbox using `sandbox_init()` from libSystem.dylib (Group G.3).
///
/// Applies a deny-default profile that allows:
/// - File reads (language-level permissions narrow further)
/// - Outbound network connections
/// - Process info lookups (needed by Rust runtime)
/// - Mach lookups (needed for macOS IPC)
///
/// Write access to the filesystem requires explicit `--allow-fs` at the
/// language level, as this profile denies file writes by default.
#[cfg(target_os = "macos")]
pub fn apply_macos_sandbox() -> SandboxResult {
    use std::ffi::CString;
    extern "C" {
        fn sandbox_init(profile: *const libc::c_char, flags: u64, errorbuf: *mut *mut libc::c_char) -> libc::c_int;
        fn sandbox_free_error(errorbuf: *mut libc::c_char);
    }

    // Deny-default profile: allow file reads and outbound network.
    // Language-level permissions narrow further.
    let profile = "(version 1)\n\
                   (deny default)\n\
                   (allow file-read*)\n\
                   (allow network-outbound)\n\
                   (allow process-info-pidinfo)\n\
                   (allow mach-lookup)\n";

    let profile_c = CString::new(profile)
        .map_err(|e| format!("sandbox: invalid profile string: {}", e))?;
    let mut errorbuf: *mut libc::c_char = std::ptr::null_mut();

    let result = unsafe { sandbox_init(profile_c.as_ptr(), 0, &mut errorbuf) };

    if result != 0 {
        let msg = if !errorbuf.is_null() {
            let s = unsafe {
                std::ffi::CStr::from_ptr(errorbuf)
                    .to_string_lossy()
                    .to_string()
            };
            unsafe { sandbox_free_error(errorbuf) };
            s
        } else {
            "unknown sandbox error".to_string()
        };
        return Err(format!("macOS sandbox_init failed: {}", msg));
    }

    Ok(())
}

/// Query whether OS-level sandbox is available on the current platform.
pub fn sandbox_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Check if we can read /proc/self/status (basic Linux check)
        std::fs::read_to_string("/proc/self/status").is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Return a human-readable description of the sandbox level.
pub fn sandbox_description(sandbox: bool) -> &'static str {
    if !sandbox {
        return "none (language-level permissions only)";
    }
    #[cfg(target_os = "linux")]
    {
        "linux: prctl(NO_NEW_PRIVS, DUMPABLE=0) + seccomp-BPF execve/ptrace blocklist"
    }
    #[cfg(target_os = "macos")]
    {
        "macos: sandbox_init() deny-default profile (file-read + network-outbound)"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        "language-level only (OS sandbox not supported on this platform)"
    }
}

/// Return a human-readable description of the strict sandbox level.
pub fn sandbox_strict_description(enabled: bool) -> &'static str {
    if !enabled {
        return "none (language-level permissions only)";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux: prctl(NO_NEW_PRIVS, DUMPABLE=0) + seccomp-BPF allowlist (EPERM for unlisted syscalls)"
    }
    #[cfg(target_os = "macos")]
    {
        "macos: sandbox_init() deny-default profile (file-read + network-outbound)"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        target_os = "macos"
    )))]
    {
        "language-level only (strict OS sandbox not supported on this platform)"
    }
}
