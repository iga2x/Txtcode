use crate::runtime::bytecode_vm::BytecodeVM;

/// Debugger for Txt-code programs
#[allow(dead_code)]
pub struct Debugger {
    breakpoints: Vec<usize>,
    vm: BytecodeVM,
}

impl Debugger {
    pub fn new() -> Self {
        let vm = BytecodeVM::new();
        
        Self {
            breakpoints: Vec::new(),
            vm,
        }
    }

    pub fn add_breakpoint(&mut self, address: usize) {
        self.breakpoints.push(address);
    }

    pub fn remove_breakpoint(&mut self, address: usize) {
        self.breakpoints.retain(|&a| a != address);
    }

    pub fn step(&mut self) -> Result<(), crate::runtime::RuntimeError> {
        // Execute one instruction
        // In a full implementation, this would step through bytecode
        Ok(())
    }

    pub fn continue_execution(&mut self) -> Result<crate::runtime::Value, crate::runtime::RuntimeError> {
        // TODO: Implement when BytecodeVM has execute method
        Ok(crate::runtime::Value::Null)
    }

    pub fn inspect_variable(&self, _name: &str) -> Option<crate::runtime::Value> {
        // In a full implementation, this would inspect VM state
        None
    }

    pub fn get_call_stack(&self) -> Vec<String> {
        // Return call stack
        Vec::new()
    }
}

