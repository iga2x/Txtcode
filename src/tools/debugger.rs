use crate::compiler::bytecode::Bytecode;
use crate::runtime::bytecode_vm::BytecodeVM;
use crate::runtime::{RuntimeError, Value};

/// State snapshot returned after each debug step
#[derive(Debug)]
pub struct DebugState {
    pub ip: usize,
    pub instruction: String,
    pub done: bool,
}

/// Debugger for Txt-code programs
pub struct Debugger {
    breakpoints: Vec<usize>,
    vm: BytecodeVM,
    bytecode: Option<Bytecode>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            vm: BytecodeVM::new(),
            bytecode: None,
        }
    }

    /// Load bytecode into the debugger and reset VM state
    pub fn load(&mut self, bytecode: Bytecode) {
        self.vm.reset();
        self.bytecode = Some(bytecode);
    }

    pub fn add_breakpoint(&mut self, address: usize) {
        if !self.breakpoints.contains(&address) {
            self.breakpoints.push(address);
        }
    }

    /// Set a breakpoint at a source line number.
    /// Finds the first instruction whose debug_info entry is at or after the given line.
    /// Returns the instruction index that was registered, or None if not found.
    pub fn add_breakpoint_at_line(&mut self, line: usize) -> Option<usize> {
        let debug_info = self.bytecode.as_ref().map(|b| &b.debug_info)?;
        // Find the instruction with the smallest ip whose line >= requested line
        let ip = debug_info
            .iter()
            .filter(|&&(_, l)| l >= line)
            .min_by_key(|&&(ip, _)| ip)
            .map(|&(ip, _)| ip)?;
        self.add_breakpoint(ip);
        Some(ip)
    }

    /// Return the source line number for a given instruction index (nearest preceding entry).
    pub fn source_line_for_ip(&self, ip: usize) -> Option<usize> {
        let debug_info = self.bytecode.as_ref().map(|b| &b.debug_info)?;
        debug_info
            .iter()
            .filter(|&&(i, _)| i <= ip)
            .max_by_key(|&&(i, _)| i)
            .map(|&(_, line)| line)
    }

    pub fn remove_breakpoint(&mut self, address: usize) {
        self.breakpoints.retain(|&a| a != address);
    }

    pub fn list_breakpoints(&self) -> &[usize] {
        &self.breakpoints
    }

    /// Execute one instruction and return the resulting debug state
    pub fn step(&mut self) -> Result<DebugState, RuntimeError> {
        let bytecode = self
            .bytecode
            .as_ref()
            .ok_or_else(|| RuntimeError::new("No bytecode loaded".to_string()))?;

        let ip = self.vm.get_ip();
        let instruction = if ip < bytecode.instructions.len() {
            format!("{:?}", bytecode.instructions[ip])
        } else {
            "END".to_string()
        };

        let more = self.vm.execute_single(bytecode)?;

        Ok(DebugState {
            ip,
            instruction,
            done: !more,
        })
    }

    /// Run until a breakpoint is hit or execution finishes
    pub fn continue_execution(&mut self) -> Result<Value, RuntimeError> {
        let bytecode = self
            .bytecode
            .as_ref()
            .ok_or_else(|| RuntimeError::new("No bytecode loaded".to_string()))?
            .clone();

        loop {
            let ip = self.vm.get_ip();
            if ip >= bytecode.instructions.len() {
                break;
            }
            // Check if we've hit a breakpoint (skip the initial ip on first call)
            if self.breakpoints.contains(&ip) {
                return Err(RuntimeError::new(format!("Breakpoint hit at ip={}", ip)));
            }
            let more = self.vm.execute_single(&bytecode)?;
            if !more {
                break;
            }
        }
        Ok(Value::Null)
    }

    /// Inspect a variable by name in the current VM scope
    pub fn inspect_variable(&self, name: &str) -> Option<Value> {
        self.vm.get_variable(name).cloned()
    }

    /// Return call stack frames as strings
    pub fn get_call_stack(&self) -> Vec<String> {
        self.vm.get_call_stack_frames()
    }

    /// Return the top of the operand stack, if any
    pub fn get_stack_top(&self) -> Option<Value> {
        self.vm.get_stack().last().cloned()
    }

    /// Return the entire operand stack
    pub fn get_stack(&self) -> Vec<Value> {
        self.vm.get_stack().to_vec()
    }

    /// Return all variables in scope
    pub fn get_all_variables(&self) -> std::collections::HashMap<String, Value> {
        self.vm.get_all_variables().clone()
    }

    /// Current instruction pointer
    pub fn current_ip(&self) -> usize {
        self.vm.get_ip()
    }

    /// Step over: advance until the source line changes (or execution ends).
    /// Unlike `step` which executes one instruction, `step_over` skips remaining
    /// instructions on the current source line and stops at the first instruction
    /// of the next line.
    pub fn step_over(&mut self) -> Result<DebugState, RuntimeError> {
        let bytecode = self
            .bytecode
            .as_ref()
            .ok_or_else(|| RuntimeError::new("No bytecode loaded".to_string()))?
            .clone();

        let start_line = self.source_line_for_ip(self.vm.get_ip());

        loop {
            let ip = self.vm.get_ip();
            if ip >= bytecode.instructions.len() {
                let instruction = "END".to_string();
                return Ok(DebugState { ip, instruction, done: true });
            }
            let instruction = format!("{:?}", bytecode.instructions[ip]);
            let more = self.vm.execute_single(&bytecode)?;
            if !more {
                return Ok(DebugState { ip, instruction, done: true });
            }
            let new_line = self.source_line_for_ip(self.vm.get_ip());
            if new_line != start_line {
                return Ok(DebugState { ip: self.vm.get_ip(), instruction, done: false });
            }
        }
    }

    /// Total number of instructions in loaded bytecode
    pub fn instruction_count(&self) -> usize {
        self.bytecode
            .as_ref()
            .map(|b| b.instructions.len())
            .unwrap_or(0)
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}
