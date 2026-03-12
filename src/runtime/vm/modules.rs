use super::VirtualMachine;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::tools::logger::log_debug;
use std::collections::HashMap;

/// Module import/export management methods for VirtualMachine
impl VirtualMachine {
    /// Execute an import statement
    pub fn execute_import(
        &mut self,
        modules: &[String],
        from: &Option<String>,
        alias: &Option<String>,
    ) -> Result<Value, RuntimeError> {
        let module_name = from.as_ref().ok_or_else(|| {
            self.create_error("Import statement requires 'from' clause".to_string())
        })?;

        // Resolve module path
        let module_path = self
            .module_resolver
            .resolve_module(module_name, self.current_file.as_deref())?;

        // Check for circular imports
        self.module_resolver
            .check_circular_import(&module_path, &self.import_stack)?;

        // Add to import stack
        self.import_stack.push(module_path.clone());

        // Load and execute module
        let module_program = self.module_resolver.load_module(&module_path)?;

        // Create a new scope for the module
        self.push_scope();

        // Clear exported symbols for this module
        self.exported_symbols.clear();

        // Execute module statements to populate its namespace
        for stmt in &module_program.statements {
            self.execute_statement(stmt)?;
        }

        // Collect exported items (functions, variables) from the module scope
        let module_scope = self
            .scope_manager
            .scopes()
            .last()
            .ok_or_else(|| self.create_error("Module scope not found".to_string()))?;

        // Create module namespace (Map of exported items)
        let mut module_namespace = HashMap::new();

        // If there are explicit exports, only export those
        let has_explicit_exports = !self.exported_symbols.is_empty();

        if has_explicit_exports {
            // Only export explicitly exported symbols
            for name in &self.exported_symbols {
                // Check module scope first
                if let Some(value) = module_scope.get(name) {
                    module_namespace.insert(name.clone(), value.clone());
                } else if let Some(value) = self.scope_manager.globals().get(name) {
                    // Check globals (for functions)
                    module_namespace.insert(name.clone(), value.clone());
                }
            }
        } else {
            // No explicit exports - export everything that doesn't start with "_"
            // Add items from module scope
            for (name, value) in module_scope {
                if !name.starts_with("_") {
                    module_namespace.insert(name.clone(), value.clone());
                }
            }

            // Also check for items in globals that might have been added during module execution
            for (name, value) in self.scope_manager.globals() {
                if !name.starts_with("_") && !module_namespace.contains_key(name) {
                    module_namespace.insert(name.clone(), value.clone());
                }
            }
        }

        // Pop module scope
        self.pop_scope();
        self.import_stack.pop();

        // Determine the import name
        let import_name = if let Some(alias_name) = alias {
            alias_name.clone()
        } else if modules.len() == 1 && modules[0] == "*" {
            // Import all - use module name as namespace
            module_name.clone()
        } else if modules.len() == 1 {
            // Single import
            modules[0].clone()
        } else {
            // Multiple imports - use module name as namespace
            module_name.clone()
        };

        if modules.len() == 1 && modules[0] != "*" {
            // Single named import
            let item_name = &modules[0];
            if let Some(value) = module_namespace.get(item_name) {
                // Remove const flag if it exists (imported items are not const in importing scope)
                self.scope_manager.remove_const_if_exists(&import_name);
                // Use set_variable to allow importing
                self.set_variable(import_name.clone(), value.clone())?;
                log_debug(&format!(
                    "Imported '{}' from module '{}'",
                    item_name, module_name
                ));
            } else {
                return Err(self.create_error(format!(
                    "Module '{}' does not export '{}'",
                    module_name, item_name
                )));
            }
        } else if modules.len() == 1 && modules[0] == "*" {
            // Import all - create namespace map
            let namespace_value = Value::Map(module_namespace);
            self.scope_manager.remove_const_if_exists(&import_name);
            self.set_variable(import_name.clone(), namespace_value)?;
            log_debug(&format!(
                "Imported all from module '{}' as '{}'",
                module_name, import_name
            ));
        } else {
            // Multiple imports - import each one
            for item_name in modules {
                if let Some(value) = module_namespace.get(item_name) {
                    // Remove const flag if it exists
                    self.scope_manager.remove_const_if_exists(item_name);
                    self.set_variable(item_name.clone(), value.clone())?;
                    log_debug(&format!(
                        "Imported '{}' from module '{}'",
                        item_name, module_name
                    ));
                } else {
                    return Err(self.create_error(format!(
                        "Module '{}' does not export '{}'",
                        module_name, item_name
                    )));
                }
            }
        }

        Ok(Value::Null)
    }

    /// Execute an export statement
    pub fn execute_export(&mut self, names: &[String]) -> Result<Value, RuntimeError> {
        // Add names to exported symbols set
        for name in names {
            self.exported_symbols.insert(name.clone());
            log_debug(&format!("Marked '{}' for export", name));
        }

        Ok(Value::Null)
    }
}
