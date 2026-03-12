// Tool execution results - structured output from tool execution

use std::time::SystemTime;

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: ToolOutput,
    pub duration: u64, // Execution duration in seconds
    pub timestamp: SystemTime,
}

/// Tool output - stdout, stderr, exit code
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl ToolOutput {
    pub fn new(stdout: String, stderr: String, exit_code: i32) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
        }
    }

    /// Get combined output (stdout + stderr)
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

impl ToolResult {
    /// Create a successful result
    pub fn success(tool_name: String, output: ToolOutput, duration: u64) -> Self {
        Self {
            tool_name,
            success: true,
            output,
            duration,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a failed result
    pub fn failure(tool_name: String, output: ToolOutput, duration: u64) -> Self {
        Self {
            tool_name,
            success: false,
            output,
            duration,
            timestamp: SystemTime::now(),
        }
    }
}
