use crate::runtime::core::stack::CallFrame;
use crate::runtime::core::Value;

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

/// Runtime error
#[derive(Debug)]
pub struct RuntimeError {
    message: String,
    hint: Option<String>,
    stack_trace: Vec<CallFrame>,
    /// Set only for control-flow signals; None for genuine runtime errors.
    signal: Option<ControlFlowSignal>,
}

impl RuntimeError {
    /// Create a genuine runtime error with a message.
    pub fn new(message: String) -> Self {
        Self {
            message,
            hint: None,
            stack_trace: Vec::new(),
            signal: None,
        }
    }

    // ── Control-flow signal constructors ────────────────────────────────────

    /// Wrap a `return →` value as a propagation signal.
    pub fn return_value(v: Value) -> Self {
        Self {
            message: String::new(),
            hint: None,
            stack_trace: Vec::new(),
            signal: Some(ControlFlowSignal::ReturnValue(v)),
        }
    }

    /// Create a `break` signal.
    pub fn break_signal() -> Self {
        Self {
            message: String::new(),
            hint: None,
            stack_trace: Vec::new(),
            signal: Some(ControlFlowSignal::Break),
        }
    }

    /// Create a `continue` signal.
    pub fn continue_signal() -> Self {
        Self {
            message: String::new(),
            hint: None,
            stack_trace: Vec::new(),
            signal: Some(ControlFlowSignal::Continue),
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
            write!(f, "{}", self.message)?;
            if let Some(hint) = &self.hint {
                write!(f, " ({})", hint)?;
            }
            if !self.stack_trace.is_empty() {
                write!(f, "\n\nStack trace:")?;
                for (i, frame) in self.stack_trace.iter().enumerate() {
                    write!(f, "\n  {}: {} at line {}, column {}",
                        self.stack_trace.len() - i,
                        frame.function_name,
                        frame.line,
                        frame.column)?;
                }
            }
            Ok(())
        }
    }
}

impl std::error::Error for RuntimeError {}
