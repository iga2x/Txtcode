use crate::parser::ast::{Expression, Literal, Pattern};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use indexmap::IndexMap;
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
                            Value::String(v) if v.as_ref() == string_val => {
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
                } else if let Some(dot_pos) = name.find('.') {
                    // Enum variant pattern: "Color.Red" — must match exactly, no binding
                    let enum_name = &name[..dot_pos];
                    let variant_name = &name[dot_pos + 1..];
                    match value {
                        Value::Enum(ev_enum, ev_variant, _)
                            if ev_enum == enum_name && ev_variant == variant_name =>
                        {
                            // Pattern matches — no binding needed
                        }
                        _ => {
                            return Err(self.create_error(format!(
                                "Enum pattern mismatch: expected {}.{}, got {:?}",
                                enum_name, variant_name, value
                            )));
                        }
                    }
                } else {
                    // Regular identifier pattern - always matches, binds the value
                    self.set_variable(name.clone(), value.clone())?;
                }
            }
            Pattern::Array(patterns) => {
                if let Value::Array(arr) = value {
                    // Check if the last pattern is a rest pattern (...name).
                    let rest_name = patterns.last().and_then(|p| {
                        if let Pattern::Rest(name) = p { Some(name.clone()) } else { None }
                    });
                    // Positional patterns are everything before the optional rest.
                    let positional = if rest_name.is_some() {
                        &patterns[..patterns.len() - 1]
                    } else {
                        &patterns[..]
                    };

                    // Count non-ignore positional patterns for minimum length check.
                    let required_count = positional
                        .iter()
                        .filter(|p| !matches!(p, Pattern::Ignore))
                        .count();

                    if arr.len() < required_count {
                        return Err(self.create_error(format!(
                            "Array pattern: expected at least {} elements, got {}",
                            required_count,
                            arr.len()
                        )));
                    }

                    // Bind positional patterns.
                    let mut arr_index = 0;
                    for pattern in positional.iter() {
                        if matches!(pattern, Pattern::Ignore) {
                            if arr_index < arr.len() {
                                arr_index += 1;
                            }
                        } else if arr_index < arr.len() {
                            self.bind_pattern(pattern, &arr[arr_index])?;
                            arr_index += 1;
                        } else {
                            return Err(self.create_error(
                                "Array pattern: not enough elements".to_string(),
                            ));
                        }
                    }

                    // Bind rest pattern to the remaining elements.
                    if let Some(name) = rest_name {
                        let tail: Vec<Value> = arr[arr_index..].to_vec();
                        self.set_variable(name, Value::Array(tail))?;
                    }
                } else {
                    return Err(self
                        .create_error(format!("Cannot destructure non-array value: {:?}", value)));
                }
            }
            Pattern::Rest(_) => {
                // Should never be evaluated outside of an array pattern context.
                return Err(self.create_error(
                    "Rest pattern (...name) is only valid inside an array pattern".to_string(),
                ));
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
                            let mut rest_fields = IndexMap::new();
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
                            let mut rest_fields = IndexMap::new();
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
                } else if let Value::Enum(_ev_enum, ev_variant, payload) = value {
                    // Enum constructor pattern: Circle(r) matches Value::Enum(_, "Circle", Some(r_val))
                    if ev_variant != type_name {
                        return Err(self.create_error(format!(
                            "Enum constructor pattern mismatch: expected variant '{}', got '{}'",
                            type_name, ev_variant
                        )));
                    }
                    match args.len() {
                        0 => {
                            // no payload expected
                        }
                        1 => {
                            // bind payload to single pattern arg
                            let inner = payload.as_deref().unwrap_or(&Value::Null);
                            self.bind_pattern(&args[0], inner)?;
                        }
                        _ => {
                            return Err(self.create_error(format!(
                                "Enum constructor pattern: at most 1 payload argument supported, got {}",
                                args.len()
                            )));
                        }
                    }
                } else {
                    return Err(self.create_error(format!(
                        "Constructor pattern '{}' requires a struct or enum value, got {:?}",
                        type_name, value
                    )));
                }
            }
            Pattern::Ignore => {
                // Ignore pattern - do nothing
            }
            // Task 12.5: Or-pattern — succeed if any sub-pattern matches
            Pattern::Or(pats) => {
                let mut matched = false;
                for pat in pats {
                    if self.bind_pattern(pat, value).is_ok() {
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return Err(self.create_error(format!(
                        "No branch of or-pattern matched {:?}",
                        value
                    )));
                }
            }
            // N.1: Typed literal pattern — matches a specific literal value directly.
            Pattern::Literal(lit) => {
                let matches = match (lit, value) {
                    (Literal::Integer(n), Value::Integer(v)) => v == n,
                    (Literal::Float(f), Value::Float(v)) => (v - f).abs() < f64::EPSILON,
                    (Literal::String(s), Value::String(v)) => v.as_ref() == s.as_str(),
                    (Literal::Char(c), Value::Char(v)) => v == c,
                    (Literal::Boolean(b), Value::Boolean(v)) => v == b,
                    (Literal::Null, Value::Null) => true,
                    _ => false,
                };
                if !matches {
                    return Err(self.create_error(format!(
                        "Literal pattern mismatch: expected {:?}, got {:?}",
                        lit, value
                    )));
                }
            }
            // Task 12.5: Range pattern — `start..=end`
            Pattern::Range(start_expr, end_expr) => {
                let start = self.evaluate_expression(start_expr)?;
                let end = self.evaluate_expression(end_expr)?;
                let in_range = match (value, &start, &end) {
                    (Value::Integer(v), Value::Integer(s), Value::Integer(e)) => v >= s && v <= e,
                    (Value::Float(v), Value::Float(s), Value::Float(e)) => v >= s && v <= e,
                    (Value::Integer(v), Value::Float(s), Value::Float(e)) => {
                        (*v as f64) >= *s && (*v as f64) <= *e
                    }
                    (Value::Float(v), Value::Integer(s), Value::Integer(e)) => {
                        *v >= (*s as f64) && *v <= (*e as f64)
                    }
                    _ => false,
                };
                if !in_range {
                    return Err(self.create_error(format!(
                        "Value {:?} is not in range {:?}..={:?}",
                        value, start, end
                    )));
                }
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
            Expression::Propagate { value, .. } => {
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
