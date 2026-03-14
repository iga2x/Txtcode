use crate::parser::ast::{Expression, Pattern};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use std::collections::{HashMap, HashSet};

use super::VirtualMachine;

/// Helper functions for VirtualMachine
impl VirtualMachine {
    /// Bind a pattern to a value (destructuring)
    pub(super) fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        value: &Value,
    ) -> Result<(), RuntimeError> {
        match pattern {
            Pattern::Identifier(name) => {
                // Check if this is a literal pattern (starts with "__literal_")
                if name.starts_with("__literal_") {
                    // Extract the literal value from the pattern name
                    let literal_str = name
                        .strip_prefix("__literal_")
                        .ok_or_else(|| {
                            self.create_error(format!(
                                "Internal: expected __literal_ prefix in '{}'",
                                name
                            ))
                        })?;

                    // Parse and match against the actual value
                    // Try to parse as integer
                    if let Ok(pattern_int) = literal_str.parse::<i64>() {
                        match value {
                            Value::Integer(v) if *v == pattern_int => {
                                // Literal matches - pattern matches, no binding needed
                                return Ok(());
                            }
                            _ => {
                                return Err(self.create_error(format!(
                                    "Literal pattern mismatch: expected {}, got {:?}",
                                    pattern_int, value
                                )));
                            }
                        }
                    }
                    // Try to parse as float
                    else if let Ok(pattern_float) = literal_str.parse::<f64>() {
                        match value {
                            Value::Float(v) if (v - pattern_float).abs() < f64::EPSILON => {
                                return Ok(());
                            }
                            _ => {
                                return Err(self.create_error(format!(
                                    "Literal pattern mismatch: expected {}, got {:?}",
                                    pattern_float, value
                                )));
                            }
                        }
                    }
                    // Check for boolean/null literals
                    else if literal_str == "true" {
                        match value {
                            Value::Boolean(true) => return Ok(()),
                            _ => {
                                return Err(self.create_error(format!(
                                    "Literal pattern mismatch: expected true, got {:?}",
                                    value
                                )));
                            }
                        }
                    } else if literal_str == "false" {
                        match value {
                            Value::Boolean(false) => return Ok(()),
                            _ => {
                                return Err(self.create_error(format!(
                                    "Literal pattern mismatch: expected false, got {:?}",
                                    value
                                )));
                            }
                        }
                    } else if literal_str == "null" {
                        match value {
                            Value::Null => return Ok(()),
                            _ => {
                                return Err(self.create_error(format!(
                                    "Literal pattern mismatch: expected null, got {:?}",
                                    value
                                )));
                            }
                        }
                    }
                    // Try as string (remove quotes if present)
                    else {
                        let string_val = if (literal_str.starts_with('"')
                            && literal_str.ends_with('"'))
                            || (literal_str.starts_with('\'') && literal_str.ends_with('\''))
                        {
                            &literal_str[1..literal_str.len() - 1]
                        } else {
                            literal_str
                        };

                        match value {
                            Value::String(v) if v == string_val => {
                                return Ok(());
                            }
                            _ => {
                                return Err(self.create_error(format!(
                                    "Literal pattern mismatch: expected \"{}\", got {:?}",
                                    string_val, value
                                )));
                            }
                        }
                    }
                } else {
                    // Regular identifier pattern - always matches, binds the value
                    self.set_variable(name.clone(), value.clone())?;
                }
            }
            Pattern::Array(patterns) => {
                if let Value::Array(arr) = value {
                    // Count non-ignore patterns to determine minimum required elements
                    let required_count = patterns
                        .iter()
                        .filter(|p| !matches!(p, Pattern::Ignore))
                        .count();

                    // Check if we have enough elements (at least as many as non-ignore patterns)
                    if arr.len() < required_count {
                        return Err(self.create_error(format!(
                            "Array destructuring: expected at least {} elements (ignoring _ patterns), got {}",
                            required_count,
                            arr.len()
                        )));
                    }

                    // Bind patterns, skipping ignore patterns but still consuming array elements
                    let mut arr_index = 0;
                    for pattern in patterns.iter() {
                        if matches!(pattern, Pattern::Ignore) {
                            // Skip this element if it's an ignore pattern
                            if arr_index < arr.len() {
                                arr_index += 1;
                            }
                        } else {
                            // Bind the pattern to the current array element
                            if arr_index < arr.len() {
                                self.bind_pattern(pattern, &arr[arr_index])?;
                                arr_index += 1;
                            } else {
                                return Err(self.create_error(
                                    "Array destructuring: not enough elements for pattern"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                } else {
                    return Err(self
                        .create_error(format!("Cannot destructure non-array value: {:?}", value)));
                }
            }
            Pattern::Struct { fields, rest } => {
                match value {
                    Value::Struct(_, struct_fields) => {
                        // Bind each field
                        for (field_name, pattern) in fields {
                            if let Some(field_value) = struct_fields.get(field_name) {
                                self.bind_pattern(pattern, field_value)?;
                            } else {
                                return Err(self.create_error(format!(
                                    "Struct field '{}' not found in destructuring",
                                    field_name
                                )));
                            }
                        }

                        // Handle rest pattern
                        if let Some(rest_name) = rest {
                            let mut rest_fields = HashMap::new();
                            for (field_name, field_value) in struct_fields {
                                // Only include fields not already destructured
                                if !fields.iter().any(|(name, _)| name == field_name) {
                                    rest_fields.insert(field_name.clone(), field_value.clone());
                                }
                            }
                            self.set_variable(rest_name.clone(), Value::Map(rest_fields))?;
                        }
                    }
                    Value::Map(map) => {
                        // Bind each field
                        for (field_name, pattern) in fields {
                            if let Some(field_value) = map.get(field_name) {
                                self.bind_pattern(pattern, field_value)?;
                            } else {
                                return Err(self.create_error(format!(
                                    "Map key '{}' not found in destructuring",
                                    field_name
                                )));
                            }
                        }

                        // Handle rest pattern
                        if let Some(rest_name) = rest {
                            let mut rest_fields = HashMap::new();
                            for (field_name, field_value) in map {
                                // Only include fields not already destructured
                                if !fields.iter().any(|(name, _)| name == field_name) {
                                    rest_fields.insert(field_name.clone(), field_value.clone());
                                }
                            }
                            self.set_variable(rest_name.clone(), Value::Map(rest_fields))?;
                        }
                    }
                    _ => {
                        return Err(self.create_error(format!(
                            "Cannot destructure non-struct/map value: {:?}",
                            value
                        )));
                    }
                }
            }
            Pattern::Constructor { type_name, args } => {
                // Constructor pattern: Point(10, 20) or Point(x, y)
                // Match against struct values
                if let Value::Struct(struct_type, struct_fields) = value {
                    // Check if type names match
                    if struct_type != type_name {
                        return Err(self.create_error(format!(
                            "Type mismatch: expected {}, got {}",
                            type_name, struct_type
                        )));
                    }

                    // Get struct definition to know field order
                    // Clone the field names to avoid borrow checker issues
                    let field_names: Vec<String> =
                        if let Some(field_defs) = self.struct_defs.get(type_name) {
                            // Match arguments to fields in order
                            if args.len() != field_defs.len() {
                                return Err(self.create_error(format!(
                                "Constructor pattern argument count mismatch: expected {}, got {}",
                                field_defs.len(), args.len()
                            )));
                            }
                            field_defs.iter().map(|(name, _)| name.clone()).collect()
                        } else {
                            Vec::new()
                        };

                    if !field_names.is_empty() {
                        // Bind each argument pattern to the corresponding field value
                        for (idx, field_name) in field_names.iter().enumerate() {
                            if let Some(field_value) = struct_fields.get(field_name) {
                                if idx < args.len() {
                                    self.bind_pattern(&args[idx], field_value)?;
                                } else {
                                    return Err(self.create_error(
                                        "Constructor pattern: not enough arguments".to_string(),
                                    ));
                                }
                            } else if idx < struct_fields.len() {
                                // Try by position if field name doesn't match
                                let field_values: Vec<_> = struct_fields.values().collect();
                                if idx < field_values.len() && idx < args.len() {
                                    self.bind_pattern(&args[idx], field_values[idx])?;
                                } else {
                                    return Err(self.create_error(format!(
                                        "Constructor pattern: field '{}' not found",
                                        field_name
                                    )));
                                }
                            } else {
                                return Err(self.create_error(format!(
                                    "Constructor pattern: field '{}' not found",
                                    field_name
                                )));
                            }
                        }
                    } else {
                        // No struct definition found - try to match by field count
                        // This is a fallback for when struct def might not be registered
                        if args.len() != struct_fields.len() {
                            return Err(self.create_error(format!(
                                "Constructor pattern argument count mismatch: expected {}, got {}",
                                struct_fields.len(),
                                args.len()
                            )));
                        }

                        // Bind arguments to fields in order (assuming same order)
                        let field_values: Vec<_> = struct_fields.values().collect();
                        for (pattern, field_value) in args.iter().zip(field_values.iter()) {
                            self.bind_pattern(pattern, field_value)?;
                        }
                    }
                } else {
                    return Err(self.create_error(format!(
                        "Constructor pattern '{}' requires a struct value, got {:?}",
                        type_name, value
                    )));
                }
            }
            Pattern::Ignore => {
                // Ignore pattern - do nothing
            }
        }

        Ok(())
    }

    /// Extract free variables (variables used but not defined) from an expression
    pub(super) fn extract_free_variables(
        expr: &Expression,
        param_names: &HashSet<String>,
    ) -> HashSet<String> {
        let mut free_vars = HashSet::new();

        match expr {
            Expression::Identifier(name) => {
                if !param_names.contains(name) {
                    free_vars.insert(name.clone());
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                free_vars.extend(Self::extract_free_variables(left, param_names));
                free_vars.extend(Self::extract_free_variables(right, param_names));
            }
            Expression::UnaryOp { operand, .. } => {
                free_vars.extend(Self::extract_free_variables(operand, param_names));
            }
            Expression::FunctionCall { arguments, .. } => {
                for arg in arguments {
                    free_vars.extend(Self::extract_free_variables(arg, param_names));
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    free_vars.extend(Self::extract_free_variables(elem, param_names));
                }
            }
            Expression::Map { entries, .. } => {
                for (k, v) in entries {
                    free_vars.extend(Self::extract_free_variables(k, param_names));
                    free_vars.extend(Self::extract_free_variables(v, param_names));
                }
            }
            Expression::Set { elements, .. } => {
                for elem in elements {
                    free_vars.extend(Self::extract_free_variables(elem, param_names));
                }
            }
            Expression::Index { target, index, .. } => {
                free_vars.extend(Self::extract_free_variables(target, param_names));
                free_vars.extend(Self::extract_free_variables(index, param_names));
            }
            Expression::Member { target, .. } => {
                free_vars.extend(Self::extract_free_variables(target, param_names));
            }
            Expression::Lambda { params, body, .. } => {
                let mut lambda_params = param_names.clone();
                for param in params {
                    lambda_params.insert(param.name.clone());
                }
                free_vars.extend(Self::extract_free_variables(body, &lambda_params));
            }
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                free_vars.extend(Self::extract_free_variables(condition, param_names));
                free_vars.extend(Self::extract_free_variables(true_expr, param_names));
                free_vars.extend(Self::extract_free_variables(false_expr, param_names));
            }
            Expression::Slice {
                target,
                start,
                end,
                step,
                ..
            } => {
                free_vars.extend(Self::extract_free_variables(target, param_names));
                if let Some(s) = start {
                    free_vars.extend(Self::extract_free_variables(s, param_names));
                }
                if let Some(e) = end {
                    free_vars.extend(Self::extract_free_variables(e, param_names));
                }
                if let Some(st) = step {
                    free_vars.extend(Self::extract_free_variables(st, param_names));
                }
            }
            Expression::InterpolatedString { segments, .. } => {
                use crate::parser::ast::InterpolatedSegment;
                for segment in segments {
                    if let InterpolatedSegment::Expression(expr) = segment {
                        free_vars.extend(Self::extract_free_variables(expr, param_names));
                    }
                }
            }
            Expression::Await { expression, .. } => {
                free_vars.extend(Self::extract_free_variables(expression, param_names));
            }
            Expression::OptionalMember { target, .. } => {
                free_vars.extend(Self::extract_free_variables(target, param_names));
            }
            Expression::OptionalCall {
                target, arguments, ..
            } => {
                free_vars.extend(Self::extract_free_variables(target, param_names));
                for arg in arguments {
                    free_vars.extend(Self::extract_free_variables(arg, param_names));
                }
            }
            Expression::OptionalIndex { target, index, .. } => {
                free_vars.extend(Self::extract_free_variables(target, param_names));
                free_vars.extend(Self::extract_free_variables(index, param_names));
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                free_vars.extend(Self::extract_free_variables(object, param_names));
                for arg in arguments {
                    free_vars.extend(Self::extract_free_variables(arg, param_names));
                }
            }
            Expression::Literal(_) => {
                // Literals don't have free variables
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    free_vars.extend(Self::extract_free_variables(field_expr, param_names));
                }
            }
            Expression::Spread { value, .. } => {
                free_vars.extend(Self::extract_free_variables(value, param_names));
            }
        }

        free_vars
    }

    /// Capture the current environment for given variable names
    pub(super) fn capture_environment(
        &self,
        var_names: &HashSet<String>,
    ) -> HashMap<String, Value> {
        let mut captured = HashMap::new();
        for name in var_names {
            if let Some(val) = self.get_variable(name) {
                captured.insert(name.clone(), val);
            }
        }
        captured
    }
}
