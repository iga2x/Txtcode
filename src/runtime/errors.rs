use crate::runtime::core::stack::CallFrame;
use crate::runtime::core::Value;

/// Maximum call stack depth before raising a recursion error.
///
/// Kept at 50 because the AST VM uses Rust's own call stack for Txtcode
/// recursion (each Txtcode frame = ~10-20 Rust frames in debug mode).
/// Increasing this limit without first making the VM iterative causes
/// Rust's thread stack to overflow before our guard fires.
///
/// TODO (Group 7): convert AST VM to iterative execution, then raise to 500+.
/// The value is configurable via `RuntimeConfig::max_call_depth` for release
/// builds where frame sizes are much smaller.
pub const MAX_CALL_DEPTH: usize = 50;

/// Control-flow signals — non-error exits that must pierce block boundaries.
/// These are not runtime errors; they ride the `Result<_, RuntimeError>` channel
/// so that Rust's `?` operator propagates them automatically through every nested
/// executor without requiring signature changes.
#[derive(Debug)]
pub enum ControlFlowSignal {
    /// `return →` — carry the return value to the enclosing function boundary.
    ReturnValue(Value),
    /// `break` — terminate the nearest enclosing loop.
    Break,
    /// `continue` — skip the rest of the current loop iteration.
    Continue,
}

/// Stable machine-readable error codes for IDE and CI consumers.
/// Codes are stable across patch versions; new codes are additive only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// General unclassified runtime error (catch-all).
    E0000,
    /// Permission denied (any resource).
    E0001,
    /// Capability token denied.
    E0002,
    /// Rate limit exceeded.
    E0003,
    /// Intent declaration violation.
    E0004,
    /// Undefined variable or function.
    E0010,
    /// Type mismatch at runtime.
    E0011,
    /// Division by zero.
    E0012,
    /// Index out of bounds.
    E0013,
    /// Maximum call depth exceeded.
    E0014,
    /// Execution timeout.
    E0020,
    /// Memory limit exceeded.
    E0021,
    /// File system error.
    E0030,
    /// Network error.
    E0031,
    /// OS / process error.
    E0032,
    /// Import / module error.
    E0040,
    /// Struct field type mismatch at construction or assignment.
    E0016,
    /// Async/await used without --experimental flag.
    E0051,
    /// Experimental feature disabled.
    E0052,
    /// Cryptographic operation failed.
    E0050,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::E0000 => "E0000",
            Self::E0001 => "E0001",
            Self::E0002 => "E0002",
            Self::E0003 => "E0003",
            Self::E0004 => "E0004",
            Self::E0010 => "E0010",
            Self::E0011 => "E0011",
            Self::E0012 => "E0012",
            Self::E0013 => "E0013",
            Self::E0014 => "E0014",
            Self::E0020 => "E0020",
            Self::E0021 => "E0021",
            Self::E0030 => "E0030",
            Self::E0031 => "E0031",
            Self::E0032 => "E0032",
            Self::E0040 => "E0040",
            Self::E0016 => "E0016",
            Self::E0051 => "E0051",
            Self::E0052 => "E0052",
            Self::E0050 => "E0050",
        }
    }

    /// Infer an error code from an error message string.
    pub fn infer_from_message(msg: &str) -> Self {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("permission") || lower.contains("access denied") {
            Self::E0001
        } else if lower.contains("capability") {
            Self::E0002
        } else if lower.contains("rate limit") {
            Self::E0003
        } else if lower.contains("intent") {
            Self::E0004
        } else if lower.contains("undefined") || lower.contains("not defined") {
            Self::E0010
        } else if lower.contains("struct field") && (lower.contains("type") || lower.contains("expected")) {
            Self::E0016
        } else if lower.contains("async") && (lower.contains("experimental") || lower.contains("not supported")) {
            Self::E0051
        } else if lower.contains("experimental") && lower.contains("disabled") {
            Self::E0052
        } else if lower.contains("type") && (lower.contains("mismatch") || lower.contains("expected")) {
            Self::E0011
        } else if lower.contains("division by zero") || lower.contains("divide by zero") {
            Self::E0012
        } else if lower.contains("index out of bounds") || lower.contains("out of range") {
            Self::E0013
        } else if lower.contains("call depth") || lower.contains("stack overflow") || lower.contains("recursion") {
            Self::E0014
        } else if lower.contains("timeout") || lower.contains("timed out") {
            Self::E0020
        } else if lower.contains("memory limit") {
            Self::E0021
        } else if lower.contains("file") || lower.contains("io error") || lower.contains("path") {
            Self::E0030
        } else if lower.contains("network") || lower.contains("connect") || lower.contains("http") {
            Self::E0031
        } else if lower.contains("process") || lower.contains("exec") || lower.contains("spawn") {
            Self::E0032
        } else if lower.contains("import") || lower.contains("module") {
            Self::E0040
        } else if lower.contains("encrypt") || lower.contains("decrypt") || lower.contains("crypto") || lower.contains("hmac") {
            Self::E0050
        } else {
            Self::E0000
        }
    }
}

/// Runtime error
#[derive(Debug)]
pub struct RuntimeError {
    message: String,
    hint: Option<String>,
    stack_trace: Vec<CallFrame>,
    /// Set only for control-flow signals; None for genuine runtime errors.
    signal: Option<ControlFlowSignal>,
    /// Machine-readable error code for IDE and CI consumers.
    pub code: Option<ErrorCode>,
    /// Source location where the error originated: (line, column).
    /// None when the error comes from internal/stdlib code with no AST span.
    pub span: Option<(usize, usize)>,
}

impl RuntimeError {
    /// Create a genuine runtime error with a message.
    /// The error code is inferred automatically from the message content.
    pub fn new(message: String) -> Self {
        let code = ErrorCode::infer_from_message(&message);
        Self {
            code: Some(code),
            message,
            hint: None,
            stack_trace: Vec::new(),
            signal: None,
            span: None,
        }
    }

    /// Attach a source location to this error (line, column).
    /// No-op if the error already has a span (first wins — the innermost site).
    pub fn with_span(mut self, line: usize, col: usize) -> Self {
        if self.span.is_none() {
            self.span = Some((line, col));
        }
        self
    }

    /// Attach an explicit error code, overriding the inferred one.
    pub fn with_code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    // ── Control-flow signal constructors ────────────────────────────────────

    /// Wrap a `return →` value as a propagation signal.
    pub fn return_value(v: Value) -> Self {
        Self {
            code: None,
            message: String::new(),
            hint: None,
            stack_trace: Vec::new(),
            signal: Some(ControlFlowSignal::ReturnValue(v)),
            span: None,
        }
    }

    /// Create a `break` signal.
    pub fn break_signal() -> Self {
        Self {
            code: None,
            message: String::new(),
            hint: None,
            stack_trace: Vec::new(),
            signal: Some(ControlFlowSignal::Break),
            span: None,
        }
    }

    /// Create a `continue` signal.
    pub fn continue_signal() -> Self {
        Self {
            code: None,
            message: String::new(),
            hint: None,
            stack_trace: Vec::new(),
            signal: Some(ControlFlowSignal::Continue),
            span: None,
        }
    }

    // ── Signal predicates ────────────────────────────────────────────────────

    /// True for any control-flow signal (return / break / continue).
    /// try/catch blocks use this to decide whether to bypass the catch handler.
    pub fn is_control_flow_signal(&self) -> bool {
        self.signal.is_some()
    }

    pub fn is_break_signal(&self) -> bool {
        matches!(self.signal, Some(ControlFlowSignal::Break))
    }

    pub fn is_continue_signal(&self) -> bool {
        matches!(self.signal, Some(ControlFlowSignal::Continue))
    }

    /// If this is a ReturnValue signal, extract the value and return Ok(v).
    /// Otherwise return Err(self) so the caller can re-propagate.
    pub fn take_return_value(self) -> Result<Value, Self> {
        match self.signal {
            Some(ControlFlowSignal::ReturnValue(v)) => Ok(v),
            _ => Err(self),
        }
    }

    // ── Existing accessors ───────────────────────────────────────────────────

    pub fn with_hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn with_stack_trace(mut self, stack_trace: Vec<CallFrame>) -> Self {
        self.stack_trace = stack_trace;
        self
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(signal) = &self.signal {
            // Should never be displayed in production — signals are always caught
            // at their respective boundaries before being shown to users.
            write!(f, "ControlFlowSignal({:?})", signal)
        } else {
            // Prefix with error code and source location when available.
            if let Some(code) = &self.code {
                write!(f, "[{}] ", code.as_str())?;
            }
            if let Some((line, col)) = self.span {
                write!(f, "line {}:{} — ", line, col)?;
            }
            write!(f, "{}", self.message)?;
            if let Some(hint) = &self.hint {
                write!(f, " ({})", hint)?;
            }
            if !self.stack_trace.is_empty() {
                write!(f, "\n\nStack trace:")?;
                for (i, frame) in self.stack_trace.iter().enumerate() {
                    write!(
                        f,
                        "\n  {}: {} at line {}, column {}",
                        self.stack_trace.len() - i,
                        frame.function_name,
                        frame.line,
                        frame.column
                    )?;
                }
            }
            Ok(())
        }
    }
}

impl std::error::Error for RuntimeError {}
