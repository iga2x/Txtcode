// Execution trace graph - structured representation of execution

use std::collections::HashMap;
use std::time::SystemTime;

/// Execution trace - graph of execution nodes and edges
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    pub nodes: Vec<TraceNode>,
    pub edges: Vec<(usize, usize)>, // (from_id, to_id) - execution flow edges
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            start_time: SystemTime::now(),
            end_time: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            edges: Vec::with_capacity(capacity),
            start_time: SystemTime::now(),
            end_time: None,
        }
    }

    /// Finalize trace (set end time)
    pub fn finalize(&mut self) {
        self.end_time = Some(SystemTime::now());
    }

    /// Get execution duration in milliseconds
    pub fn duration_ms(&self) -> u128 {
        let end = self.end_time.unwrap_or_else(SystemTime::now);
        end.duration_since(self.start_time)
            .unwrap_or_default()
            .as_millis()
    }

    /// Get execution duration in seconds
    pub fn duration_secs(&self) -> u64 {
        let end = self.end_time.unwrap_or_else(SystemTime::now);
        end.duration_since(self.start_time)
            .unwrap_or_default()
            .as_secs()
    }
}

impl Default for ExecutionTrace {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace node - represents a single execution event
#[derive(Debug, Clone)]
pub struct TraceNode {
    pub id: usize,
    pub node_type: TraceNodeType,
    pub timestamp: SystemTime,
    pub variables: HashMap<String, VariableState>, // Variable state at this point
}

/// Trace node type - different types of execution events
#[derive(Debug, Clone)]
pub enum TraceNodeType {
    Statement {
        statement: String,
        success: bool,
        error: Option<String>,
    },
    Expression {
        expression: String,
        value: Option<String>,
        error: Option<String>,
    },
    VariableAssignment {
        variable: String,
        value: String,
        scope: String,
    },
    FunctionCall {
        function: String,
        arguments: Vec<String>,
        result: Option<String>,
        error: Option<String>,
    },
    ControlFlow {
        kind: String, // "if", "while", "for", "match", etc.
        condition: String,
        taken: bool,
    },
    Entry {
        location: String, // Function name, file, etc.
    },
    Exit {
        location: String,
        result: Option<String>,
    },
}

/// Variable state at a point in execution
#[derive(Debug, Clone)]
pub struct VariableState {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub scope: String, // "global", "local", function name, etc.
    pub timestamp: SystemTime,
}
