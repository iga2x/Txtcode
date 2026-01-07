use crate::parser::ast::*;
use crate::typecheck::types::{TypeContext, FunctionType};

#[derive(Debug, Clone)]
pub struct TypeChecker {
    context: TypeContext,
    errors: Vec<TypeError>,
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: crate::lexer::Span,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
            errors: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.errors.clear();

        for statement in &program.statements {
            self.check_statement(statement);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Expression(expr) => {
                self.check_expression(expr);
            }
            Statement::Assignment { name, type_annotation, value, .. } => {
                let value_type = self.check_expression(value);
                
                if let Some(annotated_type) = type_annotation {
                    if !value_type.is_compatible_with(annotated_type) {
                        self.errors.push(TypeError {
                            message: format!(
                                "Type mismatch: expected {}, got {}",
                                self.type_to_string(annotated_type),
                                self.type_to_string(&value_type)
                            ),
                            span: value.span(),
                        });
                    } else {
                        self.context.define_variable(name.clone(), annotated_type.clone());
                    }
                } else {
                    // Type inference
                    self.context.define_variable(name.clone(), value_type);
                }
            }
            Statement::FunctionDef { name, params, return_type, body, .. } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.type_annotation.clone().unwrap_or(Type::Int) // Default to int if not annotated
                    })
                    .collect();

                let func_return_type = return_type.clone().unwrap_or(Type::Int);
                
                let func_type = FunctionType {
                    params: param_types.clone(),
                    return_type: Box::new(func_return_type.clone()),
                };

                self.context.define_function(name.clone(), func_type);

                // Check function body in new context
                let mut body_context = TypeContext::new();
                for (param, param_type) in params.iter().zip(param_types.iter()) {
                    body_context.define_variable(param.name.clone(), param_type.clone());
                }

                let old_context = std::mem::replace(&mut self.context, body_context);
                
                for stmt in body {
                    self.check_statement(stmt);
                }

                // Check return type
                if let Some(Statement::Return { value, .. }) = body.last() {
                    if let Some(return_expr) = value {
                        let return_type_actual = self.check_expression(return_expr);
                        if !return_type_actual.is_compatible_with(&func_return_type) {
                            self.errors.push(TypeError {
                                message: format!(
                                    "Return type mismatch: expected {}, got {}",
                                    self.type_to_string(&func_return_type),
                                    self.type_to_string(&return_type_actual)
                                ),
                                span: return_expr.span(),
                            });
                        }
                    }
                }

                self.context = old_context;
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.check_expression(expr);
                }
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, .. } => {
                let cond_type = self.check_expression(condition);
                if !cond_type.is_compatible_with(&Type::Bool) {
                    self.errors.push(TypeError {
                        message: "If condition must be boolean".to_string(),
                        span: condition.span(),
                    });
                }

                for stmt in then_branch {
                    self.check_statement(stmt);
                }

                for (elseif_cond, elseif_body) in else_if_branches {
                    let elseif_type = self.check_expression(elseif_cond);
                    if !elseif_type.is_compatible_with(&Type::Bool) {
                        self.errors.push(TypeError {
                            message: "Elseif condition must be boolean".to_string(),
                            span: elseif_cond.span(),
                        });
                    }
                    for stmt in elseif_body {
                        self.check_statement(stmt);
                    }
                }

                if let Some(else_body) = else_branch {
                    for stmt in else_body {
                        self.check_statement(stmt);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                let cond_type = self.check_expression(condition);
                if !cond_type.is_compatible_with(&Type::Bool) {
                    self.errors.push(TypeError {
                        message: "While condition must be boolean".to_string(),
                        span: condition.span(),
                    });
                }

                for stmt in body {
                    self.check_statement(stmt);
                }
            }
            Statement::For { iterable, body, .. } => {
                let iter_type = self.check_expression(iterable);
                match iter_type {
                    Type::Array(_) => {}
                    _ => {
                        self.errors.push(TypeError {
                            message: "For loop requires an array".to_string(),
                            span: iterable.span(),
                        });
                    }
                }

                for stmt in body {
                    self.check_statement(stmt);
                }
            }
            Statement::Repeat { count, body, .. } => {
                let count_type = self.check_expression(count);
                if !count_type.is_compatible_with(&Type::Int) {
                    self.errors.push(TypeError {
                        message: "Repeat count must be an integer".to_string(),
                        span: count.span(),
                    });
                }

                for stmt in body {
                    self.check_statement(stmt);
                }
            }
            Statement::Match { value, cases, .. } => {
                let _value_type = self.check_expression(value);
                
                for case in cases {
                    if let Some(guard) = &case.guard {
                        let guard_type = self.check_expression(guard);
                        if !guard_type.is_compatible_with(&Type::Bool) {
                            self.errors.push(TypeError {
                                message: "Case guard must be boolean".to_string(),
                                span: guard.span(),
                            });
                        }
                    }

                    for stmt in &case.body {
                        self.check_statement(stmt);
                    }
                }
            }
            Statement::Try { body, catch, .. } => {
                for stmt in body {
                    self.check_statement(stmt);
                }

                if let Some((_, catch_body)) = catch {
                    for stmt in catch_body {
                        self.check_statement(stmt);
                    }
                }
            }
            Statement::Break { .. } | Statement::Continue { .. } | Statement::Import { .. } => {
                // These don't need type checking
            }
        }
    }

    fn check_expression(&mut self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(lit) => self.literal_type(lit),
            Expression::Identifier(name) => {
                self.context
                    .get_variable(name)
                    .cloned()
                    .unwrap_or_else(|| {
                        self.errors.push(TypeError {
                            message: format!("Undefined variable: {}", name),
                            span: expr.span(),
                        });
                        Type::Int // Default type for errors
                    })
            }
            Expression::BinaryOp { left, op, right, .. } => {
                let left_type = self.check_expression(left);
                let right_type = self.check_expression(right);
                self.binary_op_type(op, &left_type, &right_type, expr)
            }
            Expression::UnaryOp { op, operand, .. } => {
                let operand_type = self.check_expression(operand);
                self.unary_op_type(op, &operand_type, expr)
            }
            Expression::FunctionCall { name, arguments, .. } => {
                let arg_types: Vec<Type> = arguments.iter().map(|arg| self.check_expression(arg)).collect();
                
                if let Some(func_type) = self.context.get_function(name) {
                    if func_type.params.len() != arg_types.len() {
                        self.errors.push(TypeError {
                            message: format!(
                                "Function {} expects {} arguments, got {}",
                                name,
                                func_type.params.len(),
                                arg_types.len()
                            ),
                            span: expr.span(),
                        });
                        return Type::Int;
                    }

                    // Check argument types
                    for (param_type, arg_type) in func_type.params.iter().zip(arg_types.iter()) {
                        if !arg_type.is_compatible_with(param_type) {
                            self.errors.push(TypeError {
                                message: format!(
                                    "Argument type mismatch: expected {}, got {}",
                                    self.type_to_string(param_type),
                                    self.type_to_string(arg_type)
                                ),
                                span: expr.span(),
                            });
                        }
                    }

                    *func_type.return_type.clone()
                } else {
                    // Built-in functions
                    self.builtin_function_type(name, &arg_types, expr)
                }
            }
            Expression::Array { elements, .. } => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Int)) // Default to int array
                } else {
                    let first_type = self.check_expression(&elements[0]);
                    for element in elements.iter().skip(1) {
                        let elem_type = self.check_expression(element);
                        if !elem_type.is_compatible_with(&first_type) {
                            self.errors.push(TypeError {
                                message: "Array elements must have the same type".to_string(),
                                span: element.span(),
                            });
                        }
                    }
                    Type::Array(Box::new(first_type))
                }
            }
            Expression::Map { entries, .. } => {
                if entries.is_empty() {
                    Type::Map(Box::new(Type::String)) // Default to string map
                } else {
                    let first_value_type = self.check_expression(&entries[0].1);
                    for (_, value_expr) in entries.iter().skip(1) {
                        let value_type = self.check_expression(value_expr);
                        if !value_type.is_compatible_with(&first_value_type) {
                            self.errors.push(TypeError {
                                message: "Map values must have the same type".to_string(),
                                span: value_expr.span(),
                            });
                        }
                    }
                    Type::Map(Box::new(first_value_type))
                }
            }
            Expression::Index { target, index, .. } => {
                let target_type = self.check_expression(target);
                let index_type = self.check_expression(index);

                match target_type {
                    Type::Array(element_type) => {
                        if !index_type.is_compatible_with(&Type::Int) {
                            self.errors.push(TypeError {
                                message: "Array index must be an integer".to_string(),
                                span: index.span(),
                            });
                        }
                        *element_type
                    }
                    Type::Map(value_type) => {
                        if !index_type.is_compatible_with(&Type::String) {
                            self.errors.push(TypeError {
                                message: "Map key must be a string".to_string(),
                                span: index.span(),
                            });
                        }
                        *value_type
                    }
                    _ => {
                        self.errors.push(TypeError {
                            message: "Cannot index this type".to_string(),
                            span: target.span(),
                        });
                        Type::Int
                    }
                }
            }
            Expression::Member { target, .. } => {
                let target_type = self.check_expression(target);
                match target_type {
                    Type::Map(value_type) => *value_type,
                    _ => {
                        self.errors.push(TypeError {
                            message: "Cannot access member on this type".to_string(),
                            span: target.span(),
                        });
                        Type::Int
                    }
                }
            }
            Expression::Lambda { params, body, .. } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| p.type_annotation.clone().unwrap_or(Type::Int))
                    .collect();

                let return_type = self.check_expression(body);
                Type::Function {
                    params: param_types,
                    return_type: Box::new(return_type),
                }
            }
        }
    }

    fn literal_type(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Integer(_) => Type::Int,
            Literal::Float(_) => Type::Float,
            Literal::String(_) => Type::String,
            Literal::Boolean(_) => Type::Bool,
            Literal::Null => Type::Identifier("null".to_string()),
        }
    }

    fn binary_op_type(&mut self, op: &BinaryOperator, left: &Type, right: &Type, expr: &Expression) -> Type {
        match op {
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
                match (left, right) {
                    (Type::Int, Type::Int) => Type::Int,
                    (Type::Float, Type::Float) => Type::Float,
                    (Type::Int, Type::Float) | (Type::Float, Type::Int) => Type::Float,
                    (Type::String, _) | (_, Type::String) if matches!(op, BinaryOperator::Add) => Type::String,
                    _ => {
                        self.errors.push(TypeError {
                            message: format!("Cannot apply {:?} to {} and {}", op, self.type_to_string(left), self.type_to_string(right)),
                            span: expr.span(),
                        });
                        Type::Int
                    }
                }
            }
            BinaryOperator::Modulo => {
                if left.is_compatible_with(&Type::Int) && right.is_compatible_with(&Type::Int) {
                    Type::Int
                } else {
                    self.errors.push(TypeError {
                        message: "Modulo requires integers".to_string(),
                        span: expr.span(),
                    });
                    Type::Int
                }
            }
            BinaryOperator::Power => {
                if (left.is_compatible_with(&Type::Int) || left.is_compatible_with(&Type::Float)) &&
                   (right.is_compatible_with(&Type::Int) || right.is_compatible_with(&Type::Float)) {
                    Type::Float
                } else {
                    self.errors.push(TypeError {
                        message: "Power requires numbers".to_string(),
                        span: expr.span(),
                    });
                    Type::Int
                }
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => Type::Bool,
            BinaryOperator::Less | BinaryOperator::Greater | BinaryOperator::LessEqual | BinaryOperator::GreaterEqual => {
                if (left.is_compatible_with(&Type::Int) || left.is_compatible_with(&Type::Float)) &&
                   (right.is_compatible_with(&Type::Int) || right.is_compatible_with(&Type::Float)) {
                    Type::Bool
                } else {
                    self.errors.push(TypeError {
                        message: "Comparison requires numbers".to_string(),
                        span: expr.span(),
                    });
                    Type::Bool
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if left.is_compatible_with(&Type::Bool) && right.is_compatible_with(&Type::Bool) {
                    Type::Bool
                } else {
                    self.errors.push(TypeError {
                        message: "Logical operators require booleans".to_string(),
                        span: expr.span(),
                    });
                    Type::Bool
                }
            }
            BinaryOperator::BitAnd | BinaryOperator::BitOr | BinaryOperator::BitXor | BinaryOperator::LeftShift | BinaryOperator::RightShift => {
                if left.is_compatible_with(&Type::Int) && right.is_compatible_with(&Type::Int) {
                    Type::Int
                } else {
                    self.errors.push(TypeError {
                        message: "Bitwise operations require integers".to_string(),
                        span: expr.span(),
                    });
                    Type::Int
                }
            }
            BinaryOperator::Arrow => {
                // Function call - return type depends on function
                Type::Int // Simplified
            }
        }
    }

    fn unary_op_type(&mut self, op: &UnaryOperator, operand: &Type, expr: &Expression) -> Type {
        match op {
            UnaryOperator::Not => {
                if operand.is_compatible_with(&Type::Bool) {
                    Type::Bool
                } else {
                    self.errors.push(TypeError {
                        message: "Logical not requires boolean".to_string(),
                        span: expr.span(),
                    });
                    Type::Bool
                }
            }
            UnaryOperator::Minus => {
                if operand.is_compatible_with(&Type::Int) {
                    Type::Int
                } else if operand.is_compatible_with(&Type::Float) {
                    Type::Float
                } else {
                    self.errors.push(TypeError {
                        message: "Negation requires a number".to_string(),
                        span: expr.span(),
                    });
                    Type::Int
                }
            }
            UnaryOperator::BitNot => {
                if operand.is_compatible_with(&Type::Int) {
                    Type::Int
                } else {
                    self.errors.push(TypeError {
                        message: "Bitwise not requires integer".to_string(),
                        span: expr.span(),
                    });
                    Type::Int
                }
            }
        }
    }

    fn builtin_function_type(&mut self, name: &str, arg_types: &[Type], expr: &Expression) -> Type {
        match name {
            "print" => {
                if arg_types.is_empty() {
                    self.errors.push(TypeError {
                        message: "print requires at least one argument".to_string(),
                        span: expr.span(),
                    });
                }
                Type::Int // print returns nothing (represented as int for now)
            }
            _ => {
                self.errors.push(TypeError {
                    message: format!("Unknown function: {}", name),
                    span: expr.span(),
                });
                Type::Int
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
