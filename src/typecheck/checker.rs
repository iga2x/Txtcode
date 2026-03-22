use crate::parser::ast::*;
use crate::typecheck::inference::TypeInference;
use crate::typecheck::types::{constraint_allowed_types, type_constraint_name, FunctionType, Type, TypeContext};
use std::collections::HashMap;

/// Type checker for Txt-code programs
pub struct TypeChecker {
    context: TypeContext,
    errors: Vec<String>,
    /// Expected return type of the function currently being checked.
    /// `None` when checking top-level statements.
    current_return_type: Option<Type>,
    /// Task E.1: User-defined protocol names (from `protocol → Name ... end`).
    /// When a generic constraint matches a user protocol we emit a warning
    /// instead of an error (static verification of protocol compliance is not
    /// possible without full type inference).
    known_protocols: std::collections::HashSet<String>,
    /// N.2: protocol_name → required method names
    protocol_method_names: HashMap<String, Vec<String>>,
    /// N.2: struct_name → method names provided by impl blocks
    struct_methods: HashMap<String, std::collections::HashSet<String>>,
    /// N.2: struct_name → protocol names it declares to implement
    struct_protocols: HashMap<String, Vec<String>>,
    /// Q.1: Null-flow narrowing — variables narrowed within the current branch.
    /// When `if x != null { ... }`, `x` is narrowed from `T?` to `T` in the body.
    /// Restored to the original type after exiting the branch.
    narrowed: HashMap<String, Type>,
    /// Q.2: Struct field types: struct_name → [(field_name, field_type)]
    struct_field_types: HashMap<String, Vec<(String, Type)>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
            errors: Vec::new(),
            current_return_type: None,
            known_protocols: std::collections::HashSet::new(),
            protocol_method_names: HashMap::new(),
            struct_methods: HashMap::new(),
            struct_protocols: HashMap::new(),
            narrowed: HashMap::new(),
            struct_field_types: HashMap::new(),
        }
    }

    /// Type check a program (advisory — collects all errors, does not halt early).
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<String>> {
        self.errors.clear();

        // First pass: collect function signatures and enum definitions
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

    /// K.2: Strict type check — halts at the first error.
    /// Used by --strict-types: the program exits before execution when this returns Err.
    pub fn check_strict(&mut self, program: &Program) -> Result<(), String> {
        self.errors.clear();

        for statement in &program.statements {
            self.collect_declarations(statement);
        }

        for statement in &program.statements {
            self.check_statement(statement);
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
        }

        Ok(())
    }

    fn collect_declarations(&mut self, stmt: &Statement) {
        match stmt {
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
                        // K.1: Unknown instead of Int — unannotated params skip type enforcement
                        p.type_annotation.clone().unwrap_or(Type::Unknown)
                    })
                    .collect();

                // K.1: Unknown instead of Int for unannotated return type
                let return_ty = return_type.clone().unwrap_or(Type::Unknown);

                let generic_params: Vec<String> = type_params.iter().map(|tp| tp.name.clone()).collect();
                let generic_constraints: std::collections::HashMap<String, String> = type_params
                    .iter()
                    .filter_map(|tp| tp.constraint.as_ref().map(|c| (tp.name.clone(), c.clone())))
                    .collect();

                let func_type = FunctionType {
                    params: param_types,
                    return_type: Box::new(return_ty),
                    generic_params,
                    generic_constraints,
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
                        // K.1: Unknown instead of Int — do not enforce type for uninferrable consts
                        self.context.define_variable(name.clone(), Type::Unknown);
                    }
                }
            }
            // Task E.1 + N.2: register user-defined protocol names and their required methods
            Statement::Protocol { name, methods, .. } => {
                self.known_protocols.insert(name.clone());
                let method_names: Vec<String> = methods.iter().map(|(m, _, _)| m.clone()).collect();
                self.protocol_method_names.insert(name.clone(), method_names);
            }
            // N.2: record which protocols each struct declares to implement
            // Q.2: record struct field types for construction-time enforcement
            Statement::Struct { name, implements, fields, .. } => {
                if !implements.is_empty() {
                    self.struct_protocols.insert(name.clone(), implements.clone());
                }
                let field_types: Vec<(String, Type)> = fields.iter().map(|(fname, ftype)| {
                    (fname.clone(), ftype.clone())
                }).collect();
                self.struct_field_types.insert(name.clone(), field_types);
            }
            // N.2: record methods provided by impl blocks
            Statement::Impl { struct_name, methods, .. } => {
                let entry = self.struct_methods.entry(struct_name.clone()).or_default();
                for method_stmt in methods {
                    if let Statement::FunctionDef { name, .. } = method_stmt {
                        entry.insert(name.clone());
                    }
                }
            }
            // K.3: Register enum variants for exhaustiveness checking
            Statement::Enum { name, variants, .. } => {
                let variant_names: Vec<String> =
                    variants.iter().map(|(v, _)| v.clone()).collect();
                self.context.define_enum(name.clone(), variant_names);
                // Define the enum name as a variable so `EnumName.Variant` member-access
                // expressions don't trigger "undefined variable: EnumName" false positives.
                self.context.define_variable(name.clone(), Type::Unknown);
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

                // Q.2: recurse into value expression for struct literal field checks etc.
                self.check_expression_stmt(value);

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
                name,
                params,
                return_type,
                body,
                ..
            } => {
                // Register the function name in the outer context so nested uses
                // (e.g. return → inner_fn) don't get "undefined variable: name".
                self.context.define_variable(name.clone(), Type::Unknown);

                // Create new scope for function
                let mut local_context = self.context.clone();

                // Add parameters to context — K.1: Unknown for unannotated params
                for param in params {
                    let param_type = param.type_annotation.clone().unwrap_or(Type::Unknown);
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
                else_if_branches,
                ..
            } => {
                // Helper: check one (condition, body) branch
                let check_branch_condition = |errors: &mut Vec<String>, ctx: &TypeContext, cond: &Expression| {
                    let mut inference = TypeInference::new();
                    inference.context = ctx.clone();
                    match inference.infer_expression(cond) {
                        crate::typecheck::types::InferenceResult::Known(ty) => {
                            if !ty.is_compatible_with(&Type::Bool) {
                                errors.push("If condition must be boolean".to_string());
                            }
                        }
                        crate::typecheck::types::InferenceResult::Error(msg) => {
                            errors.push(format!("If condition error: {}", msg));
                        }
                        _ => {}
                    }
                };

                check_branch_condition(&mut self.errors, &self.context, condition);

                // Q.1: Null-flow narrowing
                let null_check_narrowing = self.extract_null_check(condition);
                if let Some((var_name, narrowed_type)) = &null_check_narrowing {
                    self.narrowed.insert(var_name.clone(), narrowed_type.clone());
                    let saved_ctx = self.context.clone();
                    self.context.define_variable(var_name.clone(), narrowed_type.clone());
                    for stmt in then_branch {
                        self.check_statement(stmt);
                    }
                    self.narrowed.remove(var_name);
                    self.context = saved_ctx;
                } else {
                    for stmt in then_branch {
                        self.check_statement(stmt);
                    }
                }

                // G1 fix: check all elseif branches (previously ignored with `..`)
                for (elseif_cond, elseif_body) in else_if_branches {
                    check_branch_condition(&mut self.errors, &self.context, elseif_cond);
                    for stmt in elseif_body {
                        self.check_statement(stmt);
                    }
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
                variable,
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

                // Define the loop variable so body statements don't get "undefined variable"
                let saved_ctx = self.context.clone();
                self.context.define_variable(variable.clone(), Type::Unknown);

                // Check body
                for stmt in body {
                    self.check_statement(stmt);
                }

                // Restore context (loop var scoped to body)
                self.context = saved_ctx;
            }
            // K.3: Enum exhaustiveness checking for match statements
            Statement::Match { value, cases, default, .. } => {
                self.check_match_exhaustiveness(value, cases, default);
            }
            // N.2: Protocol compliance — verify struct provides all required methods
            Statement::Struct { name, .. } => {
                if let Some(proto_names) = self.struct_protocols.get(name).cloned() {
                    let provided = self.struct_methods.get(name).cloned()
                        .unwrap_or_default();
                    for proto_name in &proto_names {
                        if let Some(required) = self.protocol_method_names.get(proto_name).cloned() {
                            for method in &required {
                                if !provided.contains(method) {
                                    self.errors.push(format!(
                                        "Struct '{}' implements protocol '{}' but is missing method '{}'",
                                        name, proto_name, method
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Statement::Expression(expr) => {
                self.check_expression_stmt(expr);
            }
            Statement::CompoundAssignment { name, value, .. } => {
                // Type-check the value expression (e.g. x += bad_expr)
                self.check_expression_stmt(value);
                // Validate RHS type is compatible with variable type
                if let Some(var_type) = self.context.get_variable(name).cloned() {
                    if var_type != Type::Unknown {
                        let mut inference = TypeInference::new();
                        inference.context = self.context.clone();
                        if let crate::typecheck::types::InferenceResult::Known(rhs_type) =
                            inference.infer_expression(value)
                        {
                            if !rhs_type.is_compatible_with(&var_type) {
                                self.errors.push(format!(
                                    "Compound assignment type mismatch: '{}' is {}, cannot assign {}",
                                    name,
                                    self.type_to_string(&var_type),
                                    self.type_to_string(&rhs_type),
                                ));
                            }
                        }
                    }
                }
            }
            Statement::IndexAssignment { target, index, value, .. } => {
                self.check_expression_stmt(index);
                self.check_expression_stmt(value);
                if let Expression::Identifier(var_name) = target {
                    if let Some(container_type) = self.context.get_variable(var_name).cloned() {
                        let mut inference = TypeInference::new();
                        inference.context = self.context.clone();
                        let val_inferred = inference.infer_expression(value);
                        match container_type {
                            Type::Array(elem_type) => {
                                if let crate::typecheck::types::InferenceResult::Known(actual) = val_inferred {
                                    if !actual.is_compatible_with(&elem_type) {
                                        self.errors.push(format!(
                                            "Index assignment type mismatch: array[{}] cannot hold {}",
                                            self.type_to_string(&elem_type),
                                            self.type_to_string(&actual),
                                        ));
                                    }
                                }
                            }
                            Type::Map(val_type) => {
                                if let crate::typecheck::types::InferenceResult::Known(actual) = val_inferred {
                                    if !actual.is_compatible_with(&val_type) {
                                        self.errors.push(format!(
                                            "Index assignment type mismatch: map[{}] cannot hold {}",
                                            self.type_to_string(&val_type),
                                            self.type_to_string(&actual),
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── K.3: Enum exhaustiveness checking ────────────────────────────────────────

    /// Check that a match on a known enum covers all variants (or has a wildcard arm).
    fn check_match_exhaustiveness(
        &mut self,
        value: &Expression,
        cases: &[(Pattern, Option<Expression>, Vec<Statement>)],
        default: &Option<Vec<Statement>>,
    ) {
        // A wildcard / default arm satisfies exhaustiveness.
        if default.is_some() {
            return;
        }

        // Determine if the subject is a known enum identifier.
        let enum_name = match value {
            Expression::Identifier(name) => {
                // Look up what type this variable has
                if let Some(ty) = self.context.get_variable(name) {
                    match ty {
                        Type::Identifier(ident) => ident.clone(),
                        _ => return, // not an enum type we know about
                    }
                } else {
                    return; // unknown variable — skip
                }
            }
            Expression::FunctionCall { name, .. } => name.clone(), // e.g. Color::Red pattern
            _ => return, // complex expression — skip
        };

        let all_variants = match self.context.get_enum_variants(&enum_name) {
            Some(v) => v.clone(),
            None => return, // not a registered enum — skip
        };

        // Collect which variant names the arms cover.
        let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (pattern, _guard, _body) in cases {
            match pattern {
                Pattern::Identifier(name) => {
                    // Plain identifier arm — may be a variant name or a wildcard binding
                    if all_variants.contains(name) {
                        covered.insert(name.clone());
                    } else if name == "_" {
                        // explicit underscore — treat as wildcard
                        return;
                    }
                }
                Pattern::Constructor { type_name, .. } => {
                    covered.insert(type_name.clone());
                }
                Pattern::Ignore => return, // wildcard — exhaustive
                Pattern::Literal(_) => {} // literal patterns don't cover enum variants
                _ => {} // complex pattern — we can't prove coverage, skip
            }
        }

        let missing: Vec<&String> = all_variants
            .iter()
            .filter(|v| !covered.contains(*v))
            .collect();

        if !missing.is_empty() {
            let missing_list = missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            self.errors.push(format!(
                "Non-exhaustive match on enum '{}': missing variants: {}",
                enum_name, missing_list
            ));
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
                // Arity check + generic type consistency
                if let Some(func_type) = self.context.get_function(name).cloned() {
                    let expected = func_type.params.len();
                    let got = arguments.len();
                    if got != expected {
                        self.errors.push(format!(
                            "Arity mismatch calling '{}': expected {} argument(s), got {}",
                            name, expected, got
                        ));
                    }
                    // Generic call-site type consistency check (Task 14.1)
                    if !func_type.generic_params.is_empty() {
                        self.check_generic_call(name, &func_type, arguments);
                    }
                }

                // Task 10.2: validate element type for stdlib array mutation calls.
                // push(arr, value) / array_push(arr, value) — 2nd arg must match arr's element type.
                // insert(arr, index, value) / array_insert(arr, idx, value) — 3rd arg.
                let (arr_arg_idx, val_arg_idx) = match name.as_str() {
                    "push" | "array_push" if arguments.len() >= 2 => (0, 1),
                    "insert" | "array_insert" if arguments.len() >= 3 => (0, 2),
                    _ => (usize::MAX, usize::MAX),
                };
                if arr_arg_idx != usize::MAX {
                    if let Expression::Identifier(arr_name) = &arguments[arr_arg_idx] {
                        if let Some(arr_type) = self.context.get_variable(arr_name).cloned() {
                            if let Type::Array(elem_type) = arr_type {
                                let mut inference = TypeInference::new();
                                inference.context = self.context.clone();
                                if let crate::typecheck::types::InferenceResult::Known(actual) =
                                    inference.infer_expression(&arguments[val_arg_idx])
                                {
                                    if !actual.is_compatible_with(&elem_type) {
                                        self.errors.push(format!(
                                            "Cannot push {} into array[{}]: type mismatch",
                                            self.type_to_string(&actual),
                                            self.type_to_string(&elem_type),
                                        ));
                                    }
                                }
                            }
                        }
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
            // Q.2: Struct literal construction — check field types match declarations.
            Expression::StructLiteral { name, fields, .. } => {
                let field_pairs: Vec<(String, Expression)> = fields.iter()
                    .map(|(fname, fexpr)| (fname.clone(), fexpr.clone()))
                    .collect();
                self.check_struct_literal_fields(name, &field_pairs);
                // Recurse into field expressions
                for (_, fexpr) in fields {
                    self.check_expression_stmt(fexpr);
                }
            }
            // N.3: Optional chaining — recurse into sub-expressions.
            Expression::OptionalMember { target, .. } => {
                self.check_expression_stmt(target);
            }
            Expression::OptionalCall { target, arguments, .. } => {
                self.check_expression_stmt(target);
                for arg in arguments {
                    self.check_expression_stmt(arg);
                }
            }
            Expression::OptionalIndex { target, index, .. } => {
                self.check_expression_stmt(target);
                self.check_expression_stmt(index);
            }
            // Recurse into container literals
            Expression::Array { elements, .. } => {
                for elem in elements {
                    self.check_expression_stmt(elem);
                }
            }
            Expression::Map { entries, .. } => {
                for (k, v) in entries {
                    self.check_expression_stmt(k);
                    self.check_expression_stmt(v);
                }
            }
            Expression::Set { elements, .. } => {
                for elem in elements {
                    self.check_expression_stmt(elem);
                }
            }
            // Recurse into member/index access
            Expression::Member { target, .. } => {
                self.check_expression_stmt(target);
            }
            Expression::Index { target, index, .. } => {
                self.check_expression_stmt(target);
                self.check_expression_stmt(index);
            }
            Expression::Slice { target, start, end, step, .. } => {
                self.check_expression_stmt(target);
                if let Some(e) = start { self.check_expression_stmt(e); }
                if let Some(e) = end   { self.check_expression_stmt(e); }
                if let Some(e) = step  { self.check_expression_stmt(e); }
            }
            // Unary op — recurse into operand
            Expression::UnaryOp { operand, .. } => {
                self.check_expression_stmt(operand);
            }
            // Ternary — check condition + both branches; warn on branch type mismatch
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                self.check_expression_stmt(condition);
                self.check_expression_stmt(true_expr);
                self.check_expression_stmt(false_expr);
                let mut inference = TypeInference::new();
                inference.context = self.context.clone();
                let t1 = inference.infer_expression(true_expr);
                let t2 = inference.infer_expression(false_expr);
                if let (
                    crate::typecheck::types::InferenceResult::Known(ty1),
                    crate::typecheck::types::InferenceResult::Known(ty2),
                ) = (t1, t2) {
                    if ty1 != Type::Unknown && ty2 != Type::Unknown
                        && !ty1.is_compatible_with(&ty2)
                        && !ty2.is_compatible_with(&ty1)
                    {
                        self.errors.push(format!(
                            "Ternary branch type mismatch: then={}, else={}",
                            self.type_to_string(&ty1),
                            self.type_to_string(&ty2),
                        ));
                    }
                }
            }
            Expression::Lambda { body, .. } => {
                self.check_expression_stmt(body);
            }
            Expression::MethodCall { object, arguments, .. } => {
                self.check_expression_stmt(object);
                for arg in arguments {
                    self.check_expression_stmt(arg);
                }
            }
            Expression::Propagate { value, .. } => {
                self.check_expression_stmt(value);
            }
            Expression::Spread { value, .. } => {
                self.check_expression_stmt(value);
            }
            _ => {}
        }
    }

    // ── Task 14.1: Generic call-site type consistency ────────────────────────

    /// Verify that all uses of the same type variable at a call site resolve to
    /// the same concrete type, and that any constraint bounds are satisfied.
    fn check_generic_call(&mut self, fn_name: &str, func_type: &FunctionType, arguments: &[Expression]) {
        let mut bindings: HashMap<String, Type> = HashMap::new();
        let mut inference = TypeInference::new();
        inference.context = self.context.clone();

        for (i, param_ty) in func_type.params.iter().enumerate() {
            if i >= arguments.len() { break; }
            if let Type::Generic(tvar) = param_ty {
                let arg_type = match inference.infer_expression(&arguments[i]) {
                    crate::typecheck::types::InferenceResult::Known(t) => t,
                    _ => continue, // unknown arg — skip
                };
                if let Some(prev) = bindings.get(tvar) {
                    if !prev.is_compatible_with(&arg_type) {
                        self.errors.push(format!(
                            "Generic type mismatch in call to '{}': type variable '{}' bound to '{}' but argument {} has type '{}'",
                            fn_name, tvar,
                            self.type_to_string(prev),
                            i + 1,
                            self.type_to_string(&arg_type),
                        ));
                    }
                } else {
                    // Check constraint
                    if let Some(constraint) = func_type.generic_constraints.get(tvar) {
                        let allowed = constraint_allowed_types(constraint);
                        let canonical = type_constraint_name(&arg_type);
                        if allowed.is_empty() {
                            // Task E.1: if it's a user-defined protocol, emit a
                            // warning rather than an error — static compliance
                            // checking requires full type inference.
                            if !self.known_protocols.contains(constraint.as_str()) {
                                self.errors.push(format!(
                                    "Unknown type constraint '{}' on type variable '{}' in '{}'",
                                    constraint, tvar, fn_name,
                                ));
                            }
                            // (user protocol constraint: accepted at type-check time)
                        } else if canonical.map_or(true, |c| !allowed.contains(c)) {
                            self.errors.push(format!(
                                "Type '{}' does not satisfy constraint '{}' on type variable '{}' in '{}'",
                                self.type_to_string(&arg_type), constraint, tvar, fn_name,
                            ));
                        }
                    }
                    bindings.insert(tvar.clone(), arg_type);
                }
            }
        }
    }

    // ── Q.1: Null-flow narrowing helpers ─────────────────────────────────────

    /// Returns (variable_name, narrowed_type) if `expr` is a null-inequality check
    /// of the form `x != null` or `null != x`.
    fn extract_null_check(&self, expr: &Expression) -> Option<(String, Type)> {
        if let Expression::BinaryOp { left, op, right, .. } = expr {
            if *op == BinaryOperator::NotEqual {
                // Pattern: `x != null`
                if let (Expression::Identifier(name), Expression::Literal(Literal::Null)) =
                    (left.as_ref(), right.as_ref())
                {
                    let base_type = self.context.get_variable(name)?;
                    if let Type::Nullable(inner) = base_type {
                        return Some((name.clone(), *inner.clone()));
                    }
                }
                // Pattern: `null != x`
                if let (Expression::Literal(Literal::Null), Expression::Identifier(name)) =
                    (left.as_ref(), right.as_ref())
                {
                    let base_type = self.context.get_variable(name)?;
                    if let Type::Nullable(inner) = base_type {
                        return Some((name.clone(), *inner.clone()));
                    }
                }
            }
        }
        None
    }

    /// Returns (variable_name) if `stmt` is a null-equality early-exit check
    /// of the form `if x == null { return }` — after the if, x is narrowed.
    #[allow(dead_code)]
    fn extract_early_return_null_check(&self, stmts: &[Statement]) -> Vec<(String, Type)> {
        let mut narrowings = Vec::new();
        for stmt in stmts {
            if let Statement::If { condition, then_branch, else_branch: None, .. } = stmt {
                // if x == null { return/... }
                if let Expression::BinaryOp { left, op, right, .. } = condition {
                    if *op == BinaryOperator::Equal {
                        let name = match (left.as_ref(), right.as_ref()) {
                            (Expression::Identifier(n), Expression::Literal(Literal::Null)) => Some(n.clone()),
                            (Expression::Literal(Literal::Null), Expression::Identifier(n)) => Some(n.clone()),
                            _ => None,
                        };
                        if let Some(name) = name {
                            // Check if body ends with return/break (early exit)
                            let ends_with_exit = then_branch.last().map_or(false, |s| {
                                matches!(s, Statement::Return { .. } | Statement::Break { .. })
                            });
                            if ends_with_exit {
                                if let Some(base_type) = self.context.get_variable(&name) {
                                    if let Type::Nullable(inner) = base_type {
                                        narrowings.push((name, *inner.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        narrowings
    }

    /// Return a TypeContext with `narrowed` overrides applied.
    #[allow(dead_code)]
    fn narrowed_context(&self) -> TypeContext {
        let mut ctx = self.context.clone();
        for (name, ty) in &self.narrowed {
            ctx.define_variable(name.clone(), ty.clone());
        }
        ctx
    }

    // ── Q.2: Struct field type check ─────────────────────────────────────────

    /// Check that a struct literal's field values match the declared field types.
    /// Also checks for missing required fields and unknown field names.
    fn check_struct_literal_fields(&mut self, struct_name: &str, fields: &[(String, Expression)]) {
        let field_types = match self.struct_field_types.get(struct_name) {
            Some(ft) => ft.clone(),
            None => return,
        };
        let mut inference = TypeInference::new();
        inference.context = self.context.clone();

        // Check each supplied field: type must match declaration
        let supplied_names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
        for (field_name, field_expr) in fields {
            if let Some((_, expected_type)) = field_types.iter().find(|(n, _)| n == field_name) {
                if *expected_type == Type::Unknown {
                    continue;
                }
                match inference.infer_expression(field_expr) {
                    crate::typecheck::types::InferenceResult::Known(actual) => {
                        if !actual.is_compatible_with(expected_type) {
                            self.errors.push(format!(
                                "struct '{}' field '{}': expected {}, got {}",
                                struct_name,
                                field_name,
                                self.type_to_string(expected_type),
                                self.type_to_string(&actual),
                            ));
                        }
                    }
                    _ => {}
                }
            } else {
                // Field not declared in struct
                self.errors.push(format!(
                    "struct '{}' has no field '{}'",
                    struct_name, field_name
                ));
            }
        }

        // Check for missing required fields (all declared fields must be supplied)
        for (declared_name, _) in &field_types {
            if !supplied_names.contains(&declared_name.as_str()) {
                self.errors.push(format!(
                    "struct '{}' missing required field '{}'",
                    struct_name, declared_name
                ));
            }
        }
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

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
