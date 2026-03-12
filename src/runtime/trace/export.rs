// Trace export - export execution trace to JSON format

use super::graph::{ExecutionTrace, TraceNode, TraceNodeType};
use crate::runtime::errors::RuntimeError;
use std::time::{SystemTime, UNIX_EPOCH};

/// Export execution trace as JSON
pub fn export_trace_json(trace: &ExecutionTrace) -> Result<String, RuntimeError> {
    use std::collections::HashMap;

    let mut json = HashMap::new();

    // Trace metadata
    let mut metadata = HashMap::new();
    metadata.insert(
        "start_time".to_string(),
        timestamp_to_string(&trace.start_time),
    );
    if let Some(ref end_time) = trace.end_time {
        metadata.insert("end_time".to_string(), timestamp_to_string(end_time));
    }
    metadata.insert("duration_ms".to_string(), trace.duration_ms().to_string());
    metadata.insert("node_count".to_string(), trace.nodes.len().to_string());
    metadata.insert("edge_count".to_string(), trace.edges.len().to_string());
    json.insert(
        "metadata".to_string(),
        serde_json::Value::Object(
            metadata
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect(),
        ),
    );

    // Nodes
    let nodes: Vec<serde_json::Value> = trace.nodes.iter().map(node_to_json).collect();
    json.insert("nodes".to_string(), serde_json::Value::Array(nodes));

    // Edges
    let edges: Vec<serde_json::Value> = trace
        .edges
        .iter()
        .map(|(from, to)| {
            serde_json::json!({
                "from": from,
                "to": to,
            })
        })
        .collect();
    json.insert("edges".to_string(), serde_json::Value::Array(edges));

    // Convert to JSON string
    serde_json::to_string_pretty(&json)
        .map_err(|e| RuntimeError::new(format!("Failed to serialize trace to JSON: {}", e)))
}

fn node_to_json(node: &TraceNode) -> serde_json::Value {
    let mut json = serde_json::json!({
        "id": node.id,
        "timestamp": timestamp_to_string(&node.timestamp),
    });

    // Add node type specific data
    match &node.node_type {
        TraceNodeType::Statement {
            statement,
            success,
            error,
        } => {
            json["type"] = serde_json::Value::String("statement".to_string());
            json["statement"] = serde_json::Value::String(statement.clone());
            json["success"] = serde_json::Value::Bool(*success);
            if let Some(ref err) = error {
                json["error"] = serde_json::Value::String(err.clone());
            }
        }
        TraceNodeType::Expression {
            expression,
            value,
            error,
        } => {
            json["type"] = serde_json::Value::String("expression".to_string());
            json["expression"] = serde_json::Value::String(expression.clone());
            if let Some(ref val) = value {
                json["value"] = serde_json::Value::String(val.clone());
            }
            if let Some(ref err) = error {
                json["error"] = serde_json::Value::String(err.clone());
            }
        }
        TraceNodeType::VariableAssignment {
            variable,
            value,
            scope,
        } => {
            json["type"] = serde_json::Value::String("variable_assignment".to_string());
            json["variable"] = serde_json::Value::String(variable.clone());
            json["value"] = serde_json::Value::String(value.clone());
            json["scope"] = serde_json::Value::String(scope.clone());
        }
        TraceNodeType::FunctionCall {
            function,
            arguments,
            result,
            error,
        } => {
            json["type"] = serde_json::Value::String("function_call".to_string());
            json["function"] = serde_json::Value::String(function.clone());
            json["arguments"] = serde_json::Value::Array(
                arguments
                    .iter()
                    .map(|a| serde_json::Value::String(a.clone()))
                    .collect(),
            );
            if let Some(ref res) = result {
                json["result"] = serde_json::Value::String(res.clone());
            }
            if let Some(ref err) = error {
                json["error"] = serde_json::Value::String(err.clone());
            }
        }
        TraceNodeType::ControlFlow {
            kind,
            condition,
            taken,
        } => {
            json["type"] = serde_json::Value::String("control_flow".to_string());
            json["kind"] = serde_json::Value::String(kind.clone());
            json["condition"] = serde_json::Value::String(condition.clone());
            json["taken"] = serde_json::Value::Bool(*taken);
        }
        TraceNodeType::Entry { location } => {
            json["type"] = serde_json::Value::String("entry".to_string());
            json["location"] = serde_json::Value::String(location.clone());
        }
        TraceNodeType::Exit { location, result } => {
            json["type"] = serde_json::Value::String("exit".to_string());
            json["location"] = serde_json::Value::String(location.clone());
            if let Some(ref res) = result {
                json["result"] = serde_json::Value::String(res.clone());
            }
        }
    }

    // Add variable state snapshot if available
    if !node.variables.is_empty() {
        let vars: serde_json::Map<String, serde_json::Value> = node
            .variables
            .iter()
            .map(|(name, state)| {
                (
                    name.clone(),
                    serde_json::json!({
                        "value": state.value,
                        "type": state.type_name,
                        "scope": state.scope,
                    }),
                )
            })
            .collect();
        json["variables"] = serde_json::Value::Object(vars);
    }

    json
}

fn timestamp_to_string(time: &SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
