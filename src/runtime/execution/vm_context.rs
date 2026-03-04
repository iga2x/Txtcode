use crate::parser::ast::{Expression, Statement, Pattern};
use crate::runtime::core::{Value, CallFrame};
use crate::runtime::errors::RuntimeError;
use crate::typecheck::types::Type;
use std::collections::HashMap;

/// Trait for accessing VM state during execution
/// This allows execution modules to work with VM without direct ownership
pub trait VMContext {
    fn get_variable(&self, name: &str) -> Option<Value>;
    fn set_variable(&mut self, name: String, value: Value);
    fn push_scope(&mut self);
    fn pop_scope(&mut self);
    fn bind_pattern(&mut self, pattern: &Pattern, value: &Value) -> Result<(), RuntimeError>;
    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, RuntimeError>;
    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError>;
    fn create_error(&self, message: String) -> RuntimeError;
    fn get_enum_defs(&self) -> &HashMap<String, Vec<(String, Option<Expression>)>>;
    fn get_struct_defs(&self) -> &HashMap<String, Vec<(String, Type)>>;
    fn push_call_frame(&mut self, frame: CallFrame);
    fn pop_call_frame(&mut self);
    fn get_exec_allowed(&self) -> bool;
    fn register_gc_allocation(&mut self, value: &Value);
}

