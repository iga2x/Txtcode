use crate::parser::ast::*;
use crate::typecheck::types::{TypeContext, InferenceResult};
use std::collections::HashMap;

/// Type inference engine
pub struct TypeInference {
    context: TypeContext,
    #[allow(dead_code)] // Reserved for future constraint-based type inference
    constraints: Vec<TypeConstraint>,
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub left: Type,
    pub right: Type,
}

impl TypeInference {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
            constraints: Vec::new(),
        }
    }

    /// Infer types for a program
    pub fn infer_program(&mut self, program: &Program) -> Result<HashMap<String, Type>, Vec<String>> {
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
            Statement::Assignment { name, type_annotation, value, .. } => {
                if let Some(annotated_type) = type_annotation {
                    // Type is explicitly annotated
                    types.insert(name.clone(), annotated_type.clone());
                } else {
                    // Infer type from value
                    match self.infer_expression(value) {
                        InferenceResult::Known(ty) => {
                            types.insert(name.clone(), ty);
                        }
                        InferenceResult::Unknown => {
                            return Err(format!("Cannot infer type for variable: {}", name));
                        }
                        InferenceResult::Error(msg) => {
                            return Err(msg);
                        }
                    }
                }
            }
            Statement::FunctionDef { name, params, return_type, .. } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.type_annotation.clone().unwrap_or_else(|| {
                            // Try to infer from usage
                            Type::Int // Default
                        })
                    })
                    .collect();

                let func_return = return_type.clone().unwrap_or(Type::Int);
                
                // Store function type
                let func_type = crate::typecheck::types::FunctionType {
                    params: param_types,
                    return_type: Box::new(func_return),
                };
                self.context.define_function(name.clone(), func_type);
            }
            _ => {
                // Other statements don't directly contribute to type map
            }
        }

        Ok(types)
    }

    fn infer_expression(&mut self, expr: &Expression) -> InferenceResult {
        match expr {
            Expression::Literal(lit) => {
                InferenceResult::Known(match lit {
                    Literal::Integer(_) => Type::Int,
                    Literal::Float(_) => Type::Float,
                    Literal::String(_) => Type::String,
                    Literal::Boolean(_) => Type::Bool,
                    Literal::Null => Type::Identifier("null".to_string()),
                })
            }
            Expression::Identifier(name) => {
                if let Some(ty) = self.context.get_variable(name) {
                    InferenceResult::Known(ty.clone())
                } else {
                    InferenceResult::Error(format!("Undefined variable: {}", name))
                }
            }
            Expression::BinaryOp { left, op, right, .. } => {
                let left_type = self.infer_expression(left);
                let right_type = self.infer_expression(right);

                match (left_type, right_type) {
                    (InferenceResult::Known(left_ty), InferenceResult::Known(right_ty)) => {
                        match op {
                            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
                                match (left_ty.clone(), right_ty.clone()) {
                                    (Type::Int, Type::Int) => InferenceResult::Known(Type::Int),
                                    (Type::Float, Type::Float) => InferenceResult::Known(Type::Float),
                                    (Type::Int, Type::Float) | (Type::Float, Type::Int) => InferenceResult::Known(Type::Float),
                                    (Type::String, _) | (_, Type::String) if matches!(op, BinaryOperator::Add) => {
                                        InferenceResult::Known(Type::String)
                                    }
                                    _ => InferenceResult::Error(format!("Cannot apply {:?} to {} and {}", op, self.type_to_string(&left_ty), self.type_to_string(&right_ty))),
                                }
                            }
                            BinaryOperator::Equal | BinaryOperator::NotEqual => InferenceResult::Known(Type::Bool),
                            BinaryOperator::Less | BinaryOperator::Greater | BinaryOperator::LessEqual | BinaryOperator::GreaterEqual => {
                                InferenceResult::Known(Type::Bool)
                            }
                            BinaryOperator::And | BinaryOperator::Or => InferenceResult::Known(Type::Bool),
                            _ => InferenceResult::Known(Type::Int), // Default for other operations
                        }
                    }
                    (InferenceResult::Error(msg), _) | (_, InferenceResult::Error(msg)) => InferenceResult::Error(msg),
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
                        }
                    }
                    InferenceResult::Error(msg) => InferenceResult::Error(msg),
                    InferenceResult::Unknown => InferenceResult::Unknown,
                }
            }
            Expression::FunctionCall { name, arguments: _arguments, .. } => {
                if let Some(func_type) = self.context.get_function(name) {
                    InferenceResult::Known(*func_type.return_type.clone())
                } else {
                // Built-in functions
                match name.as_str() {
                    "print" => InferenceResult::Known(Type::Int), // print returns nothing
                    _ => InferenceResult::Error(format!("Unknown function: {}", name)),
                }
                }
            }
            Expression::Array { elements, .. } => {
                if elements.is_empty() {
                    InferenceResult::Known(Type::Array(Box::new(Type::Int))) // Default
                } else {
                    let first_type = self.infer_expression(&elements[0]);
                    match first_type {
                        InferenceResult::Known(ty) => InferenceResult::Known(Type::Array(Box::new(ty))),
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
                        InferenceResult::Known(ty) => InferenceResult::Known(Type::Map(Box::new(ty))),
                        _ => first_value_type,
                    }
                }
            }
            Expression::Index { target, index: _index, .. } => {
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Array(element_type)) => {
                        InferenceResult::Known(*element_type)
                    }
                    InferenceResult::Known(Type::Map(value_type)) => {
                        InferenceResult::Known(*value_type)
                    }
                    InferenceResult::Known(_) => InferenceResult::Error("Cannot index this type".to_string()),
                    other => other,
                }
            }
            Expression::Member { target, .. } => {
                let target_type = self.infer_expression(target);
                match target_type {
                    InferenceResult::Known(Type::Map(value_type)) => InferenceResult::Known(*value_type),
                    InferenceResult::Known(_) => InferenceResult::Error("Cannot access member on this type".to_string()),
                    other => other,
                }
            }
            Expression::Lambda { params, body, .. } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Int))
                    .collect();

                let return_type = self.infer_expression(body);
                match return_type {
                    InferenceResult::Known(ty) => {
                        InferenceResult::Known(Type::Function {
                            params: param_types,
                            return_type: Box::new(ty),
                        })
                    }
                    other => other,
                }
            }
        }
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::String => "string".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Array(inner) => format!("array[{}]", self.type_to_string(inner)),
            Type::Map(inner) => format!("map[{}]", self.type_to_string(inner)),
            Type::Function { params, return_type } => {
                let param_strs: Vec<String> = params.iter().map(|p| self.type_to_string(p)).collect();
                format!("({}) -> {}", param_strs.join(", "), self.type_to_string(return_type))
            }
            Type::Identifier(name) => name.clone(),
            Type::Generic(name) => format!("<{}>", name),
        }
    }
}
