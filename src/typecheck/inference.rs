use crate::parser::ast::*;
use crate::typecheck::types::{InferenceResult, Type, TypeContext};
use std::collections::HashMap;

/// Type inference engine
pub struct TypeInference {
    pub context: TypeContext,
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInference {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
        }
    }

    /// Infer types for a program
    pub fn infer_program(
        &mut self,
        program: &Program,
    ) -> Result<HashMap<String, Type>, Vec<String>> {
        let mut type_map = HashMap::new();
        let mut errors = Vec::new();

        for statement in &program.statements {
            match self.infer_statement(statement) {
                Ok(types) => {
                    type_map.extend(types);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(type_map)
        } else {
            Err(errors)
        }
    }

    fn infer_statement(&mut self, statement: &Statement) -> Result<HashMap<String, Type>, String> {
        let mut types = HashMap::new();

        match statement {
            Statement::Assignment {
                pattern,
                type_annotation: _,
                value,
                ..
            } => {
                // Infer type from value
                let value_type = match self.infer_expression(value) {
                    InferenceResult::Known(ty) => ty,
                    InferenceResult::Unknown => {
                        return Err("Cannot infer type for assignment value".to_string());
                    }
                    InferenceResult::Error(msg) => {
                        return Err(msg);
                    }
                };

                // Bind pattern to type
                self.bind_pattern_type(pattern, &value_type, &mut types)?;
            }
            Statement::FunctionDef {
                name,
                type_params,
                params,
                return_type,
                ..
            } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        // K.1: Unknown instead of Int for unannotated params
                        p.type_annotation.clone().unwrap_or(Type::Unknown)
                    })
                    .collect();

                // K.1: Unknown instead of Int for unannotated return type
                let func_return = return_type.clone().unwrap_or(Type::Unknown);

                // Store function type (generic params tracked for call-site checking)
                let generic_params: Vec<String> = type_params.iter().map(|tp| tp.name.clone()).collect();
                let generic_constraints: std::collections::HashMap<String, String> = type_params
                    .iter()
                    .filter_map(|tp| tp.constraint.as_ref().map(|c| (tp.name.clone(), c.clone())))
                    .collect();
                let func_type = crate::typecheck::types::FunctionType {
                    params: param_types,
                    return_type: Box::new(func_return),
                    generic_params,
                    generic_constraints,
                };
                self.context.define_function(name.clone(), func_type);
            }
            _ => {
                // Other statements don't directly contribute to type map
            }
        }

        Ok(types)
    }

    pub fn infer_expression(&mut self, expr: &Expression) -> InferenceResult {
        match expr {
            Expression::Literal(lit) => InferenceResult::Known(match lit {
                Literal::Integer(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::String(_) => Type::String,
                Literal::Char(_) => Type::Char,
                Literal::Boolean(_) => Type::Bool,
                Literal::Null => Type::Null,
            }),
            Expression::Identifier(name) => {
                if let Some(ty) = self.context.get_variable(name) {
                    InferenceResult::Known(ty.clone())
                } else {
                    InferenceResult::Error(format!("Undefined variable: {}", name))
                }
            }
            Expression::BinaryOp {
                left, op, right, ..
            } => {
                let left_type = self.infer_expression(left);
                let right_type = self.infer_expression(right);

                match (left_type, right_type) {
                    (InferenceResult::Known(left_ty), InferenceResult::Known(right_ty)) => {
                        match op {
                            BinaryOperator::Add
                            | BinaryOperator::Subtract
                            | BinaryOperator::Multiply
                            | BinaryOperator::Divide => match (left_ty.clone(), right_ty.clone()) {
                                (Type::Int, Type::Int) => InferenceResult::Known(Type::Int),
                                (Type::Float, Type::Float) => InferenceResult::Known(Type::Float),
                                (Type::Int, Type::Float) | (Type::Float, Type::Int) => {
                                    InferenceResult::Known(Type::Float)
                                }
                                (Type::String, _) | (_, Type::String)
                                    if matches!(op, BinaryOperator::Add) =>
                                {
                                    InferenceResult::Known(Type::String)
                                }
                                // If either operand is Unknown, suppress the false-positive:
                                // the params are unannotated so we can't know the types.
                                (Type::Unknown, _) | (_, Type::Unknown) => {
                                    InferenceResult::Unknown
                                }
                                _ => InferenceResult::Error(format!(
                                    "Cannot apply {:?} to {} and {}",
                                    op,
                                    self.type_to_string(&left_ty),
                                    self.type_to_string(&right_ty)
                                )),
                            },
                            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                                InferenceResult::Known(Type::Bool)
                            }
                            BinaryOperator::Less
                            | BinaryOperator::Greater
                            | BinaryOperator::LessEqual
                            | BinaryOperator::GreaterEqual => InferenceResult::Known(Type::Bool),
                            BinaryOperator::And | BinaryOperator::Or => {
                                InferenceResult::Known(Type::Bool)
                            }
                            BinaryOperator::NullCoalesce => {
                                // Null coalesce returns the left type if not null, otherwise right type
                                InferenceResult::Known(left_ty)
                            }
                            _ => InferenceResult::Known(Type::Int), // Default for other operations
                        }
                    }
                    (InferenceResult::Error(msg), _) | (_, InferenceResult::Error(msg)) => {
                        InferenceResult::Error(msg)
                    }
                    _ => InferenceResult::Unknown,
                }
            }
            Expression::UnaryOp { op, operand, .. } => {
                let operand_type = self.infer_expression(operand);
                match operand_type {
                    InferenceResult::Known(ty) => {
                        match op {
                            UnaryOperator::Not => InferenceResult::Known(Type::Bool),
                            UnaryOperator::Minus => {
                                if ty.is_compatible_with(&Type::Int) {
                                    InferenceResult::Known(Type::Int)
                                } else if ty.is_compatible_with(&Type::Float) {
                                    InferenceResult::Known(Type::Float)
                                } else {
                                    InferenceResult::Error("Negation requires a number".to_string())
                                }
                            }
                            UnaryOperator::BitNot => InferenceResult::Known(Type::Int),
                            UnaryOperator::Increment | UnaryOperator::Decrement => {
                                // Increment/decrement return the same type as operand
                                InferenceResult::Known(ty)
                            }
                        }
                    }
                    InferenceResult::Error(msg) => InferenceResult::Error(msg),
                    InferenceResult::Unknown => InferenceResult::Unknown,
                }
            }
            Expression::FunctionCall {
                name,
                arguments: _arguments,
                ..
            } => {
                if let Some(func_type) = self.context.get_function(name) {
                    InferenceResult::Known(*func_type.return_type.clone())
                } else {
                    // Built-in functions
                    match name.as_str() {
                        "print" => InferenceResult::Known(Type::Int), // print returns nothing
                        "http_get" | "http_post" | "tcp_connect" => {
                            // Async stdlib functions return Future<T>
                            // For now, assume they return Future<String>
                            InferenceResult::Known(Type::Future(Box::new(Type::String)))
                        }
                        // All other unrecognised names (stdlib, external) are Unknown,
                        // NOT Error — we cannot type-check what we don't know about.
                        _ => InferenceResult::Unknown,
                    }
                }
            }
            Expression::Array { elements, .. } => {
                if elements.is_empty() {
                    InferenceResult::Known(Type::Array(Box::new(Type::Int))) // Default
                } else {
                    let first_type = self.infer_expression(&elements[0]);
                    match first_type {
                        InferenceResult::Known(ty) => {
                            InferenceResult::Known(Type::Array(Box::new(ty)))
                        }
                        _ => first_type,
                    }
                }
            }
            Expression::Map { entries, .. } => {
                if entries.is_empty() {
                    InferenceResult::Known(Type::Map(Box::new(Type::String))) // Default
                } else {
                    let first_value_type = self.infer_expression(&entries[0].1);
                    match first_value_type {
                        InferenceResult::Known(ty) => {
                            InferenceResult::Known(Type::Map(Box::new(ty)))
                        }
                        _ => first_value_type,
                    }
                }
            }
            Expression::Set { elements, .. } => {
                if elements.is_empty() {
                    InferenceResult::Known(Type::Set(Box::new(Type::Int))) // Default
                } else {
                    let first_element_type = self.infer_expression(&elements[0]);
                    match first_element_type {
                        InferenceResult::Known(ty) => {
                            InferenceResult::Known(Type::Set(Box::new(ty)))
                        }
                        _ => first_element_type,
                    }
                }
            }
            Expression::Index {
                target,
                index: _index,
                ..
            } => {
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Array(element_type)) => {
                        InferenceResult::Known(*element_type)
                    }
                    InferenceResult::Known(Type::Map(value_type)) => {
                        InferenceResult::Known(*value_type)
                    }
                    InferenceResult::Known(_) => {
                        InferenceResult::Error("Cannot index this type".to_string())
                    }
                    other => other,
                }
            }
            Expression::Member { target, .. } => {
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Map(value_type)) => {
                        InferenceResult::Known(*value_type)
                    }
                    InferenceResult::Known(_) => {
                        InferenceResult::Error("Cannot access member on this type".to_string())
                    }
                    other => other,
                }
            }
            Expression::Lambda { params, body, .. } => {
                let param_types: Vec<Type> = params
                    .iter()
                    // K.1: Unknown instead of Int for unannotated lambda params
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Unknown))
                    .collect();

                let return_type = self.infer_expression(body);
                match return_type {
                    InferenceResult::Known(ty) => InferenceResult::Known(Type::Function {
                        params: param_types,
                        return_type: Box::new(ty),
                    }),
                    other => other,
                }
            }
            Expression::Ternary {
                condition: _condition,
                true_expr,
                false_expr,
                ..
            } => {
                let true_type = self.infer_expression(true_expr);
                let false_type = self.infer_expression(false_expr);
                match (true_type, false_type) {
                    (InferenceResult::Known(ty), _) => InferenceResult::Known(ty),
                    (_, InferenceResult::Known(ty)) => InferenceResult::Known(ty),
                    (a, _) => a,
                }
            }
            Expression::Slice { target, .. } => {
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Array(element_type)) => {
                        InferenceResult::Known(Type::Array(element_type))
                    }
                    InferenceResult::Known(Type::String) => InferenceResult::Known(Type::String),
                    other => other,
                }
            }
            Expression::InterpolatedString { .. } => {
                // Interpolated strings always result in String type
                InferenceResult::Known(Type::String)
            }
            Expression::Await { expression, .. } => {
                // Await unwraps a Future<T> to T
                let future_type = self.infer_expression(expression);
                match future_type {
                    InferenceResult::Known(Type::Future(inner_type)) => {
                        // Await unwraps Future<T> to T
                        InferenceResult::Known(*inner_type)
                    }
                    InferenceResult::Known(Type::Function { return_type, .. }) => {
                        // If it's a function that returns a Future, unwrap it
                        if let Type::Future(inner) = return_type.as_ref() {
                            InferenceResult::Known(*inner.clone())
                        } else {
                            InferenceResult::Error(
                                "Cannot await non-Future return type".to_string(),
                            )
                        }
                    }
                    InferenceResult::Known(ty) => {
                        // If it's not a Future, that's an error
                        InferenceResult::Error(format!(
                            "Cannot await non-Future type: {}",
                            self.type_to_string(&ty)
                        ))
                    }
                    other => other,
                }
            }
            Expression::OptionalMember { target, .. } => {
                // Optional member returns the member type or null
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Identifier(_)) => InferenceResult::Unknown, // Can't infer member type
                    InferenceResult::Known(_) => InferenceResult::Unknown, // Return type of member access
                    other => other,
                }
            }
            Expression::OptionalCall { target, .. } => {
                // Optional call returns the function return type or null
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Function { return_type, .. }) => {
                        InferenceResult::Known(*return_type)
                    }
                    InferenceResult::Known(_) => InferenceResult::Unknown,
                    other => other,
                }
            }
            Expression::OptionalIndex { target, .. } => {
                // Optional index returns element type or null
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Array(element_type)) => {
                        InferenceResult::Known(*element_type)
                    }
                    InferenceResult::Known(Type::Map(value_type)) => {
                        InferenceResult::Known(*value_type)
                    }
                    InferenceResult::Known(_) => InferenceResult::Unknown,
                    other => other,
                }
            }
            Expression::MethodCall { .. } => InferenceResult::Unknown,
            Expression::StructLiteral { .. } => InferenceResult::Unknown,
            Expression::Spread { .. } => InferenceResult::Unknown,
            Expression::Propagate { .. } => InferenceResult::Unknown,
        }
    }

    fn bind_pattern_type(
        &self,
        pattern: &crate::parser::ast::Pattern,
        value_type: &Type,
        types: &mut HashMap<String, Type>,
    ) -> Result<(), String> {
        use crate::parser::ast::Pattern;

        match pattern {
            Pattern::Identifier(name) => {
                types.insert(name.clone(), value_type.clone());
            }
            Pattern::Array(patterns) => {
                if let Type::Array(element_type) = value_type {
                    if !patterns.is_empty() {
                        // All elements should have the same type
                        for pattern in patterns {
                            self.bind_pattern_type(pattern, element_type, types)?;
                        }
                    }
                } else {
                    return Err(format!(
                        "Cannot destructure non-array type: {:?}",
                        value_type
                    ));
                }
            }
            Pattern::Struct { fields, .. } => {
                // For struct destructuring, we'd need to know the struct type
                // For now, just bind identifiers
                if let Type::Identifier(_struct_name) = value_type {
                    // Struct field types require a struct definition lookup.
                    // Until a struct type registry is wired in, bind each field as `any`.
                    let any = Type::Identifier("any".to_string());
                    for (field_name, pattern) in fields {
                        types.insert(field_name.clone(), any.clone());
                        self.bind_pattern_type(pattern, &any, types)?;
                    }
                } else if let Type::Map(_) = value_type {
                    // Map destructuring — field type equals the map's value type.
                    // Until value-type propagation is implemented, bind each field as `any`.
                    let any = Type::Identifier("any".to_string());
                    for (field_name, pattern) in fields {
                        types.insert(field_name.clone(), any.clone());
                        self.bind_pattern_type(pattern, &any, types)?;
                    }
                } else {
                    return Err(format!(
                        "Cannot destructure non-struct/map type: {:?}",
                        value_type
                    ));
                }
            }
            Pattern::Constructor { type_name, args } => {
                // Constructor pattern: Point(10, 20) or Point(x, y)
                // Type-check constructor pattern
                if let Type::Identifier(struct_type) = value_type {
                    if struct_type != type_name {
                        return Err(format!(
                            "Type mismatch: expected {}, got {}",
                            type_name, struct_type
                        ));
                    }

                    // Constructor field types require a struct definition lookup.
                    // Until a struct registry is wired in, bind each arg as `any`.
                    let any = Type::Identifier("any".to_string());
                    for pattern in args {
                        self.bind_pattern_type(pattern, &any, types)?;
                    }
                } else {
                    return Err(format!(
                        "Constructor pattern requires struct type, got {:?}",
                        value_type
                    ));
                }
            }
            Pattern::Ignore => {
                // Ignore pattern - do nothing
            }
            Pattern::Or(pats) => {
                for pat in pats {
                    self.bind_pattern_type(pat, value_type, types)?;
                }
            }
            Pattern::Range(..) => {
                // Range pattern — no variable bindings to infer
            }
            Pattern::Literal(_) => {
                // N.1: Literal pattern — no variable bindings (match only)
            }
            Pattern::Rest(name) => {
                // Rest binds to an array of the same element type
                types.insert(name.clone(), value_type.clone());
            }
        }

        Ok(())
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Unknown => "unknown".to_string(),
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::String => "string".to_string(),
            Type::Char => "char".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Array(inner) => format!("array[{}]", self.type_to_string(inner)),
            Type::Map(inner) => format!("map[{}]", self.type_to_string(inner)),
            Type::Set(inner) => format!("set[{}]", self.type_to_string(inner)),
            Type::Function {
                params,
                return_type,
            } => {
                let param_strs: Vec<String> =
                    params.iter().map(|p| self.type_to_string(p)).collect();
                format!(
                    "({}) -> {}",
                    param_strs.join(", "),
                    self.type_to_string(return_type)
                )
            }
            Type::Future(inner) => format!("Future[{}]", self.type_to_string(inner)),
            Type::Identifier(name) => name.clone(),
            Type::Generic(name) => format!("<{}>", name),
            Type::Nullable(inner) => format!("{}?", self.type_to_string(inner)),
            Type::Null => "null".to_string(),
        }
    }
}
