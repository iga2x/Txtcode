use crate::parser::ast::*;
use crate::typecheck::inference::TypeInference;
use crate::typecheck::types::{FunctionType, Type, TypeContext};

/// Type checker for Txt-code programs
pub struct TypeChecker {
    context: TypeContext,
    errors: Vec<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
            errors: Vec::new(),
        }
    }

    /// Type check a program
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<String>> {
        self.errors.clear();

        // First pass: collect function signatures
        for statement in &program.statements {
            self.collect_declarations(statement);
        }

        // Second pass: type check all statements
        for statement in &program.statements {
            self.check_statement(statement);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn collect_declarations(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDef {
                name,
                params,
                return_type,
                ..
            } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.type_annotation.clone().unwrap_or(Type::Int) // Default to Int if not specified
                    })
                    .collect();

                let return_ty = return_type.clone().unwrap_or(Type::Int);

                let func_type = FunctionType {
                    params: param_types,
                    return_type: Box::new(return_ty),
                };

                self.context.define_function(name.clone(), func_type);
            }
            Statement::Const { name, value, .. } => {
                // Infer type from value
                let mut inference = TypeInference::new();
                match inference.infer_expression(value) {
                    crate::typecheck::types::InferenceResult::Known(ty) => {
                        self.context.define_variable(name.clone(), ty);
                    }
                    _ => {
                        // Use default type
                        self.context.define_variable(name.clone(), Type::Int);
                    }
                }
            }
            _ => {}
        }
    }

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assignment {
                pattern,
                type_annotation,
                value,
                ..
            } => {
                // Check value type
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();

                let value_type = match inference.infer_expression(value) {
                    crate::typecheck::types::InferenceResult::Known(ty) => ty,
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors.push(msg);
                        return;
                    }
                    crate::typecheck::types::InferenceResult::Unknown => {
                        self.errors
                            .push("Cannot infer type for assignment value".to_string());
                        return;
                    }
                };

                // Check type annotation if provided
                if let Some(annotated_type) = type_annotation {
                    if !value_type.is_compatible_with(annotated_type) {
                        self.errors.push(format!(
                            "Type mismatch: expected {}, got {}",
                            self.type_to_string(annotated_type),
                            self.type_to_string(&value_type)
                        ));
                    }
                }

                // Update context with variable type
                if let Pattern::Identifier(name) = pattern {
                    self.context.define_variable(name.clone(), value_type);
                }
            }
            Statement::FunctionDef {
                name: _name,
                params,
                return_type,
                body,
                ..
            } => {
                // Create new scope for function
                let mut local_context = self.context.clone();

                // Add parameters to context
                for param in params {
                    let param_type = param.type_annotation.clone().unwrap_or(Type::Int);
                    local_context.define_variable(param.name.clone(), param_type);
                }

                // Check function body
                let old_context = std::mem::replace(&mut self.context, local_context);
                for body_stmt in body {
                    self.check_statement(body_stmt);
                }
                self.context = old_context;

                // Check return type if specified
                if let Some(_expected_return) = return_type {
                    // Check if body returns correct type
                    // This is simplified - would need to check all return paths
                }
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                if let crate::typecheck::types::InferenceResult::Error(msg) =
                    inference.infer_expression(expr)
                {
                    self.errors.push(format!("Return type error: {}", msg));
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                // Check condition is boolean
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                match inference.infer_expression(condition) {
                    crate::typecheck::types::InferenceResult::Known(ty) => {
                        if !ty.is_compatible_with(&Type::Bool) {
                            self.errors.push("If condition must be boolean".to_string());
                        }
                    }
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors.push(format!("If condition error: {}", msg));
                    }
                    _ => {}
                }

                // Check branches
                for stmt in then_branch {
                    self.check_statement(stmt);
                }
                if let Some(else_branch) = else_branch {
                    for stmt in else_branch {
                        self.check_statement(stmt);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                // Check condition is boolean
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                match inference.infer_expression(condition) {
                    crate::typecheck::types::InferenceResult::Known(ty) => {
                        if !ty.is_compatible_with(&Type::Bool) {
                            self.errors
                                .push("While condition must be boolean".to_string());
                        }
                    }
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors.push(format!("While condition error: {}", msg));
                    }
                    _ => {}
                }

                // Check body
                for stmt in body {
                    self.check_statement(stmt);
                }
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                // Check body first (do-while executes body at least once)
                for stmt in body {
                    self.check_statement(stmt);
                }

                // Check condition is boolean
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                match inference.infer_expression(condition) {
                    crate::typecheck::types::InferenceResult::Known(ty) => {
                        if !ty.is_compatible_with(&Type::Bool) {
                            self.errors
                                .push("Do-while condition must be boolean".to_string());
                        }
                    }
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors
                            .push(format!("Do-while condition error: {}", msg));
                    }
                    _ => {}
                }
            }
            Statement::For {
                variable: _variable,
                iterable,
                body,
                ..
            } => {
                // Check iterable is iterable type
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                match inference.infer_expression(iterable) {
                    crate::typecheck::types::InferenceResult::Known(ty) => {
                        match ty {
                            Type::Array(_) | Type::Map(_) | Type::Set(_) | Type::String => {
                                // Valid iterable
                            }
                            _ => {
                                self.errors.push(
                                    "For loop iterable must be array, map, set, or string"
                                        .to_string(),
                                );
                            }
                        }
                    }
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors
                            .push(format!("For loop iterable error: {}", msg));
                    }
                    _ => {}
                }

                // Check body
                for stmt in body {
                    self.check_statement(stmt);
                }
            }
            Statement::Expression(expr) => {
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                if let crate::typecheck::types::InferenceResult::Error(msg) =
                    inference.infer_expression(expr)
                {
                    self.errors.push(msg);
                }
            }
            _ => {
                // Other statements - basic check
            }
        }
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
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

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
