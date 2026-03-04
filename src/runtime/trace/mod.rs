// Execution trace system - structured, replayable execution graphs
// Records execution flow for debugging, auditing, and AI feedback

pub mod recorder;
pub mod graph;
pub mod export;

pub use recorder::TraceRecorder;
pub use graph::{ExecutionTrace, TraceNode, TraceNodeType, VariableState};
pub use export::export_trace_json;

use crate::parser::ast::{Statement, Expression};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

/// Execution trace - records program execution for replay and analysis
pub struct ExecutionTraceSystem {
    recorder: TraceRecorder,
}

impl ExecutionTraceSystem {
    pub fn new() -> Self {
        Self {
            recorder: TraceRecorder::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            recorder: TraceRecorder::with_capacity(capacity),
        }
    }

    /// Record statement execution
    pub fn record_statement(&mut self, stmt: &Statement, result: Result<Value, RuntimeError>) {
        self.recorder.record_statement(stmt, result);
    }

    /// Record expression evaluation
    pub fn record_expression(&mut self, expr: &Expression, value: Result<Value, RuntimeError>) {
        self.recorder.record_expression(expr, value);
    }

    /// Record variable assignment
    pub fn record_variable_assignment(&mut self, name: String, value: Value, scope: Option<String>) {
        self.recorder.record_variable_assignment(name, value, scope);
    }

    /// Record function call
    pub fn record_function_call(&mut self, name: String, args: Vec<Value>, result: Result<Value, RuntimeError>) {
        self.recorder.record_function_call(name, args, result);
    }

    /// Record control flow decision
    pub fn record_control_flow(&mut self, kind: &str, condition: Value, taken: bool) {
        self.recorder.record_control_flow(kind, condition, taken);
    }

    /// Get execution trace
    pub fn get_trace(&self) -> &ExecutionTrace {
        self.recorder.get_trace()
    }

    /// Export trace as JSON
    pub fn export_json(&self) -> Result<String, RuntimeError> {
        export_trace_json(self.get_trace())
    }

    /// Clear trace
    pub fn clear(&mut self) {
        self.recorder.clear();
    }

    /// Check if trace is enabled
    pub fn is_enabled(&self) -> bool {
        self.recorder.is_enabled()
    }

    /// Enable/disable tracing
    pub fn set_enabled(&mut self, enabled: bool) {
        self.recorder.set_enabled(enabled);
    }
}

impl Default for ExecutionTraceSystem {
    fn default() -> Self {
        Self::new()
    }
}

