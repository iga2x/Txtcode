/// Call stack frame
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function_name: String,
    pub line: usize,
    pub column: usize,
}

/// Call stack for tracking function calls
pub struct CallStack {
    frames: Vec<CallFrame>,
}

impl CallStack {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn push(&mut self, frame: CallFrame) {
        self.frames.push(frame);
    }

    pub fn pop(&mut self) -> Option<CallFrame> {
        self.frames.pop()
    }

    pub fn frames(&self) -> &[CallFrame] {
        &self.frames
    }

    pub fn clone_frames(&self) -> Vec<CallFrame> {
        self.frames.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Get the current frame (top of stack)
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.frames.last()
    }
}

impl Default for CallStack {
    fn default() -> Self {
        Self::new()
    }
}
