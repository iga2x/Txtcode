use crate::compiler::bytecode::BytecodeProgram;
use crate::runtime::bytecode_vm::BytecodeVM;

/// Debugger for Txt-code programs
pub struct Debugger {
    breakpoints: Vec<usize>,
    vm: BytecodeVM,
}

impl Debugger {
    pub fn new(program: BytecodeProgram) -> Self {
        let mut vm = BytecodeVM::new();
        vm.load(&program);
        
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

    pub fn step(&mut self) -> Result<(), crate::runtime::vm::RuntimeError> {
        // Execute one instruction
        // In a full implementation, this would step through bytecode
        Ok(())
    }

    pub fn continue_execution(&mut self) -> Result<crate::runtime::vm::Value, crate::runtime::vm::RuntimeError> {
        // Continue until next breakpoint
        self.vm.execute()
    }

    pub fn inspect_variable(&self, _name: &str) -> Option<crate::runtime::vm::Value> {
        // In a full implementation, this would inspect VM state
        None
    }

    pub fn get_call_stack(&self) -> Vec<String> {
        // Return call stack
        Vec::new()
    }
}

