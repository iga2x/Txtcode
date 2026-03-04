// Lambda expression evaluation

use crate::parser::ast::{Expression, Statement, Span};
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use std::collections::HashSet;
use super::ExpressionVM;

pub fn evaluate_lambda<VM: ExpressionVM>(
    vm: &mut VM,
    params: &[crate::parser::ast::Parameter],
    body: &Expression,
) -> Result<Value, RuntimeError> {
    let param_names: HashSet<String> = params.iter()
        .map(|p| p.name.clone())
        .collect();
    
    // Find free variables (variables used in body but not parameters)
    let free_vars = VM::extract_free_variables(body, &param_names);
    
    // Capture the current environment for free variables
    let captured_env = vm.capture_environment(&free_vars);
    
    crate::tools::logger::log_debug(&format!(
        "Creating lambda with {} parameters, capturing {} variables: {:?}",
        params.len(),
        free_vars.len(),
        free_vars.iter().collect::<Vec<_>>()
    ));
    
    let func = Value::Function(
        "<lambda>".to_string(),
        params.to_vec(),
        vec![Statement::Return {
            value: Some(body.clone()),
            span: Span::default(),
        }],
        captured_env,
    );
    vm.gc_register_allocation(&func);
    Ok(func)
}

