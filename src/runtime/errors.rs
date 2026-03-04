use crate::runtime::core::stack::CallFrame;

/// Runtime error
#[derive(Debug)]
pub struct RuntimeError {
    message: String,
    hint: Option<String>,
    stack_trace: Vec<CallFrame>,
}

impl RuntimeError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            hint: None,
            stack_trace: Vec::new(),
        }
    }

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

impl std::error::Error for RuntimeError {}

