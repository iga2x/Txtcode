use super::VirtualMachine;
use crate::runtime::audit::AuditResult;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;
use crate::runtime::permissions::PermissionResource;
use crate::tools::logger::log_debug;
use indexmap::IndexMap;
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

        // S1: Permission check before reading module file
        let perm_result = self.check_permission_with_audit(
            &PermissionResource::FileSystem("read".to_string()),
            Some(module_path.to_string_lossy().as_ref()),
        );
        if let Err(ref e) = perm_result {
            let _ = self.audit_trail.log_action(
                "module.import".to_string(),
                module_path.to_string_lossy().to_string(),
                None,
                AuditResult::Denied,
                None, // B.1: ai_metadata removed
            );
            return Err(self.create_error(format!("Module import denied: {}", e)));
        }

        // Add to import stack
        self.import_stack.push(module_path.clone());

        // Load module AST (cached after first load)
        let module_program = self.module_resolver.load_module(&module_path)?;

        // ── Namespace isolation ────────────────────────────────────────────────
        // Snapshot the caller's globals BEFORE running the module.
        // After execution we restore this snapshot so the module cannot pollute
        // the caller's global namespace (e.g. define → foo writes to globals).
        let pre_globals: HashMap<String, Value> = self.scope_manager.globals().clone();

        // ── O.2: Permission isolation ──────────────────────────────────────────
        // Security note: each imported module runs with a SNAPSHOT of the
        // importer's permissions. Modules cannot grant or revoke permissions
        // for the importing scope. This is intentional and security-critical.
        // A malicious or buggy module cannot escalate the importer's permissions.
        let saved_permissions = self.permission_manager.clone();

        // Clear exported symbols for this module's execution
        self.exported_symbols.clear();

        // Push an isolated scope for the module
        self.push_scope();

        // Execute module statements to populate its namespace
        let exec_result = (|| -> Result<(), RuntimeError> {
            for stmt in &module_program.statements {
                self.execute_statement(stmt)?;
            }
            Ok(())
        })();

        // Collect the module's top scope (local vars, structs, enums) BEFORE popping
        let module_scope: HashMap<String, Value> = self
            .scope_manager
            .scopes()
            .last()
            .map(|s| s.clone())
            .unwrap_or_default();

        // Collect delta: symbols added to globals during module execution (functions, etc.)
        let post_globals = self.scope_manager.globals().clone();
        let delta_globals: HashMap<String, Value> = post_globals
            .into_iter()
            .filter(|(k, _)| !pre_globals.contains_key(k))
            .collect();

        // Build module namespace from scope + delta globals, respecting export declarations
        let mut module_namespace = IndexMap::new();
        let has_explicit_exports = !self.exported_symbols.is_empty();
        if has_explicit_exports {
            let exported = self.exported_symbols.clone();
            for name in &exported {
                if let Some(v) = module_scope.get(name) {
                    module_namespace.insert(name.clone(), v.clone());
                } else if let Some(v) = delta_globals.get(name) {
                    module_namespace.insert(name.clone(), v.clone());
                }
            }
        } else {
            // No explicit exports — expose everything that doesn't start with "_"
            for (k, v) in &module_scope {
                if !k.starts_with('_') {
                    module_namespace.insert(k.clone(), v.clone());
                }
            }
            for (k, v) in &delta_globals {
                if !k.starts_with('_') && !module_namespace.contains_key(k) {
                    module_namespace.insert(k.clone(), v.clone());
                }
            }
        }

        // Pop module scope
        self.pop_scope();

        // ── Restore caller globals (undo any pollution from the module) ────────
        *self.scope_manager.globals_mut() = pre_globals;

        // ── O.2: Restore parent permissions (prevent module permission escalation) ─
        self.permission_manager = saved_permissions;

        self.import_stack.pop();

        // Propagate any execution error AFTER cleanup
        exec_result?;

        // S4: Log successful module import to audit trail
        let _ = self.audit_trail.log_action(
            "module.import".to_string(),
            module_path.to_string_lossy().to_string(),
            None,
            AuditResult::Allowed,
            None, // B.1: ai_metadata removed
        );

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
