use crate::runtime::core::value::Value;
use std::collections::{HashMap, HashSet};

/// Scope manager for variable scoping
pub struct ScopeManager {
    globals: HashMap<String, Value>,
    scopes: Vec<HashMap<String, Value>>, // Stack of local scopes
    const_vars: HashSet<String>, // Track const variables (globals only for now)
}

impl ScopeManager {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            scopes: Vec::new(),
            const_vars: HashSet::new(),
        }
    }

    /// Get a variable from current scope or globals
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        // Check local scopes first (most recent first)
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val.clone());
            }
        }
        // Fall back to globals
        self.globals.get(name).cloned()
    }

    /// Set a variable in the current scope (or create new scope if needed)
    /// If variable exists in an outer scope, update it there instead
    /// Returns error if trying to reassign a const variable
    pub fn set_variable(&mut self, name: String, value: Value) -> Result<(), String> {
        // Check if it's a const variable
        if self.const_vars.contains(&name) {
            return Err(format!("Cannot reassign const variable '{}'", name));
        }
        
        // First, check if variable exists in any scope (from most recent to oldest)
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(&name) {
                scope.insert(name, value);
                return Ok(());
            }
        }
        
        // Check globals
        if self.globals.contains_key(&name) {
            self.globals.insert(name, value);
            return Ok(());
        }
        
        // Variable doesn't exist, create in current scope
        if let Some(scope) = self.scopes.last_mut() {
            // Set in current local scope
            scope.insert(name, value);
        } else {
            // No local scope, set in globals
            self.globals.insert(name, value);
        }
        Ok(())
    }

    /// Set a const variable (immutable)
    pub fn set_const(&mut self, name: String, value: Value) {
        self.const_vars.insert(name.clone());
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        } else {
            self.globals.insert(name, value);
        }
    }

    /// Check if a variable is const
    pub fn is_const(&self, name: &str) -> bool {
        self.const_vars.contains(name)
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Get reference to globals (for GC)
    pub fn globals(&self) -> &HashMap<String, Value> {
        &self.globals
    }

    /// Get reference to scopes (for GC)
    pub fn scopes(&self) -> &[HashMap<String, Value>] {
        &self.scopes
    }

    /// Get mutable reference to globals
    pub fn globals_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.globals
    }
    
    /// Remove const flag if variable exists (used for imports)
    pub fn remove_const_if_exists(&mut self, name: &str) {
        self.const_vars.remove(name);
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

