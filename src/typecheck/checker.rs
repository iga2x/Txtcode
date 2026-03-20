use crate::parser::ast::*;
use crate::typecheck::inference::TypeInference;
use crate::typecheck::types::{FunctionType, Type, TypeContext};

/// Type checker for Txt-code programs
pub struct TypeChecker {
    context: TypeContext,
    errors: Vec<String>,
    /// Expected return type of the function currently being checked.
    /// `None` when checking top-level statements.
    current_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
            errors: Vec::new(),
            current_return_type: None,
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
                    crate::typecheck::types::InferenceResult::Known(ty) => Some(ty),
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors.push(msg);
                        return;
                    }
                    crate::typecheck::types::InferenceResult::Unknown => None,
                };

                // Check type annotation if provided
                if let Some(annotated_type) = type_annotation {
                    if let Some(ref vt) = value_type {
                        if !vt.is_compatible_with(annotated_type) {
                            self.errors.push(format!(
                                "Type mismatch: expected {}, got {}",
                                self.type_to_string(annotated_type),
                                self.type_to_string(vt)
                            ));
                        }
                    }
                    // Task 10.2: enforce element types for typed Array/Map literals
                    self.check_collection_element_types(annotated_type, value, &mut inference);
                }

                // Update context with variable type
                if let Pattern::Identifier(name) = pattern {
                    if let Some(vt) = value_type {
                        self.context.define_variable(name.clone(), vt);
                    } else if let Some(ann) = type_annotation {
                        self.context.define_variable(name.clone(), ann.clone());
                    }
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

                // Swap in local context and expected return type
                let old_context = std::mem::replace(&mut self.context, local_context);
                let old_return_type = std::mem::replace(&mut self.current_return_type, return_type.clone());

                for body_stmt in body {
                    self.check_statement(body_stmt);
                }

                self.context = old_context;
                self.current_return_type = old_return_type;
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                // Task 10.3: check return type against declared return type
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                match inference.infer_expression(expr) {
                    crate::typecheck::types::InferenceResult::Known(actual_ty) => {
                        if let Some(ref expected_ty) = self.current_return_type.clone() {
                            if !actual_ty.is_compatible_with(expected_ty) {
                                self.errors.push(format!(
                                    "Return type mismatch: function declared to return {}, but returns {}",
                                    self.type_to_string(expected_ty),
                                    self.type_to_string(&actual_ty)
                                ));
                            }
                        }
                    }
                    crate::typecheck::types::InferenceResult::Error(msg) => {
                        self.errors.push(format!("Return type error: {}", msg));
                    }
                    crate::typecheck::types::InferenceResult::Unknown => {}
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
                // Task 10.3: check arity and null arithmetic
                self.check_expression_stmt(expr);
            }
            _ => {
                // Other statements - basic check
            }
        }
    }

    // ── Task 10.2: Collection element type enforcement ──────────────────────────

    /// When an assignment has a typed collection annotation (Array<T>, Map<T>),
    /// verify that each element in the literal matches the declared element type.
    fn check_collection_element_types(
        &mut self,
        annotation: &Type,
        value: &Expression,
        inference: &mut TypeInference,
    ) {
        match annotation {
            Type::Array(elem_type) => {
                if let Expression::Array { elements, .. } = value {
                    for (i, elem) in elements.iter().enumerate() {
                        if let crate::typecheck::types::InferenceResult::Known(actual) =
                            inference.infer_expression(elem)
                        {
                            if !actual.is_compatible_with(elem_type) {
                                self.errors.push(format!(
                                    "Array element type mismatch at index {}: expected {}, got {}",
                                    i,
                                    self.type_to_string(elem_type),
                                    self.type_to_string(&actual)
                                ));
                            }
                        }
                    }
                }
            }
            Type::Map(val_type) => {
                if let Expression::Map { entries, .. } = value {
                    for (i, (_key, val)) in entries.iter().enumerate() {
                        if let crate::typecheck::types::InferenceResult::Known(actual) =
                            inference.infer_expression(val)
                        {
                            if !actual.is_compatible_with(val_type) {
                                self.errors.push(format!(
                                    "Map value type mismatch at entry {}: expected {}, got {}",
                                    i,
                                    self.type_to_string(val_type),
                                    self.type_to_string(&actual)
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── Task 10.3: Expression-level checks (arity, null arithmetic) ─────────────

    fn check_expression_stmt(&mut self, expr: &Expression) {
        match expr {
            Expression::FunctionCall { name, arguments, .. } => {
                // Arity check: only when function is known (defined in same file)
                if let Some(func_type) = self.context.get_function(name).cloned() {
                    let expected = func_type.params.len();
                    let got = arguments.len();
                    if got != expected {
                        self.errors.push(format!(
                            "Arity mismatch calling '{}': expected {} argument(s), got {}",
                            name, expected, got
                        ));
                    }
                }
                // Recurse into arguments
                for arg in arguments {
                    self.check_expression_stmt(arg);
                }
            }
            Expression::BinaryOp { left, op, right, .. } => {
                // Null arithmetic warning: if either operand is definitively Null
                // and the operator is arithmetic, warn.
                let arithmetic_op = matches!(
                    op,
                    BinaryOperator::Add
                        | BinaryOperator::Subtract
                        | BinaryOperator::Multiply
                        | BinaryOperator::Divide
                        | BinaryOperator::Modulo
                );
                if arithmetic_op {
                    let mut inference = TypeInference::new();
                    inference.context = self.context.clone();
                    let lt = inference.infer_expression(left);
                    let rt = inference.infer_expression(right);
                    if matches!(lt, crate::typecheck::types::InferenceResult::Known(Type::Null)) {
                        self.errors.push(
                            "Potential null dereference in arithmetic: left operand may be null"
                                .to_string(),
                        );
                    }
                    if matches!(rt, crate::typecheck::types::InferenceResult::Known(Type::Null)) {
                        self.errors.push(
                            "Potential null dereference in arithmetic: right operand may be null"
                                .to_string(),
                        );
                    }
                }
                self.check_expression_stmt(left);
                self.check_expression_stmt(right);
            }
            _ => {}
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
