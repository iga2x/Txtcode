// Trace recorder - records execution events

use super::graph::{ExecutionTrace, TraceNode, TraceNodeType, VariableState};
use crate::parser::ast::{Expression, Statement};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use std::collections::HashMap;
use std::time::SystemTime;

/// Trace recorder - collects execution events
pub struct TraceRecorder {
    trace: ExecutionTrace,
    enabled: bool,
    variable_states: HashMap<String, VariableState>, // Track variable state across execution
}

impl TraceRecorder {
    pub fn new() -> Self {
        Self {
            trace: ExecutionTrace::new(),
            enabled: true,
            variable_states: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            trace: ExecutionTrace::with_capacity(capacity),
            enabled: true,
            variable_states: HashMap::new(),
        }
    }

    /// Record statement execution
    pub fn record_statement(&mut self, stmt: &Statement, result: Result<Value, RuntimeError>) {
        if !self.enabled {
            return;
        }

        let node_type = TraceNodeType::Statement {
            statement: format!("{:?}", stmt),
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
        };

        let node = TraceNode {
            id: self.trace.nodes.len(),
            node_type,
            timestamp: SystemTime::now(),
            variables: self.get_current_variable_snapshot(),
        };

        self.trace.nodes.push(node);
    }

    /// Record expression evaluation
    pub fn record_expression(&mut self, expr: &Expression, value: Result<Value, RuntimeError>) {
        if !self.enabled {
            return;
        }

        let node_type = TraceNodeType::Expression {
            expression: format!("{:?}", expr),
            value: value.as_ref().ok().map(|v| v.to_string()),
            error: value.as_ref().err().map(|e| e.to_string()),
        };

        let node = TraceNode {
            id: self.trace.nodes.len(),
            node_type,
            timestamp: SystemTime::now(),
            variables: self.get_current_variable_snapshot(),
        };

        self.trace.nodes.push(node);
    }

    /// Record variable assignment
    pub fn record_variable_assignment(
        &mut self,
        name: String,
        value: Value,
        scope: Option<String>,
    ) {
        if !self.enabled {
            return;
        }

        // Update variable state tracking
        let var_state = VariableState {
            name: name.clone(),
            value: value.to_string(),
            type_name: format!("{:?}", value),
            scope: scope.unwrap_or_else(|| "global".to_string()),
            timestamp: SystemTime::now(),
        };

        self.variable_states.insert(name.clone(), var_state.clone());

        let node_type = TraceNodeType::VariableAssignment {
            variable: name,
            value: value.to_string(),
            scope: var_state.scope.clone(),
        };

        let node = TraceNode {
            id: self.trace.nodes.len(),
            node_type,
            timestamp: SystemTime::now(),
            variables: self.get_current_variable_snapshot(),
        };

        self.trace.nodes.push(node);
    }

    /// Record function call
    pub fn record_function_call(
        &mut self,
        name: String,
        args: Vec<Value>,
        result: Result<Value, RuntimeError>,
    ) {
        if !self.enabled {
            return;
        }

        let node_type = TraceNodeType::FunctionCall {
            function: name.clone(),
            arguments: args.iter().map(|v| v.to_string()).collect(),
            result: result.as_ref().ok().map(|v| v.to_string()),
            error: result.as_ref().err().map(|e| e.to_string()),
        };

        let node_id = self.trace.nodes.len();
        let node = TraceNode {
            id: node_id,
            node_type,
            timestamp: SystemTime::now(),
            variables: self.get_current_variable_snapshot(),
        };

        self.trace.nodes.push(node);

        // Add edge for call graph
        if node_id > 0 {
            let parent_id = node_id - 1; // Previous node
            self.trace.edges.push((parent_id, node_id));
        }
    }

    /// Record control flow decision
    pub fn record_control_flow(&mut self, kind: &str, condition: Value, taken: bool) {
        if !self.enabled {
            return;
        }

        let node_type = TraceNodeType::ControlFlow {
            kind: kind.to_string(),
            condition: condition.to_string(),
            taken,
        };

        let node_id = self.trace.nodes.len();
        let node = TraceNode {
            id: node_id,
            node_type,
            timestamp: SystemTime::now(),
            variables: self.get_current_variable_snapshot(),
        };

        self.trace.nodes.push(node);

        // Add edge for control flow graph
        if node_id > 0 {
            let parent_id = node_id - 1; // Previous node
            self.trace.edges.push((parent_id, node_id));
        }
    }

    /// Get current variable state snapshot
    fn get_current_variable_snapshot(&self) -> HashMap<String, VariableState> {
        self.variable_states.clone()
    }

    /// Get execution trace
    pub fn get_trace(&self) -> &ExecutionTrace {
        &self.trace
    }

    /// Clear trace
    pub fn clear(&mut self) {
        self.trace.nodes.clear();
        self.trace.edges.clear();
        self.variable_states.clear();
    }

    /// Check if trace is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable/disable tracing
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for TraceRecorder {
    fn default() -> Self {
        Self::new()
    }
}
