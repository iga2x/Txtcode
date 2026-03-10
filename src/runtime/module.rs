use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime::errors::RuntimeError;
use crate::config::Config;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// Module resolver for handling imports
pub struct ModuleResolver {
    loaded_modules: HashMap<String, crate::parser::ast::Program>,
    search_paths: Vec<PathBuf>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // Add current directory
        if let Ok(cwd) = std::env::current_dir() {
            search_paths.push(cwd);
        }

        // Auto-inject local env packages path (highest priority after cwd)
        if let Some((env_dir, name, config)) = Config::load_active_env() {
            if config.settings.use_local_packages {
                let local_pkg = env_dir.join(&name).join("packages");
                if local_pkg.is_dir() {
                    search_paths.push(local_pkg);
                }
            }
        }

        // Add paths from TXTCODE_MODULE_PATH environment variable
        if let Ok(module_path) = std::env::var("TXTCODE_MODULE_PATH") {
            for path in module_path.split(':') {
                search_paths.push(PathBuf::from(path));
            }
        }

        Self {
            loaded_modules: HashMap::new(),
            search_paths,
        }
    }

    pub fn with_search_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.search_paths = paths;
        self
    }

    /// Resolve a module path to a file path
    pub fn resolve_module(&self, module_name: &str, current_file: Option<&Path>) -> Result<PathBuf, RuntimeError> {
        // Handle relative imports
        if module_name.starts_with("./") || module_name.starts_with("../") {
            if let Some(current) = current_file {
                let current_dir = current.parent()
                    .ok_or_else(|| RuntimeError::new("Cannot resolve relative import: current file has no parent directory".to_string()))?;
                let resolved = current_dir.join(module_name);
                return self.normalize_path(&resolved);
            } else {
                return Err(RuntimeError::new("Relative imports require a current file path".to_string()));
            }
        }
        
        // Try to find module in search paths
        for search_path in &self.search_paths {
            // Try with .tc extension
            let candidate = search_path.join(format!("{}.tc", module_name));
            if candidate.exists() {
                return Ok(candidate);
            }
            
            // Try without extension (if it's already .tc)
            let candidate2 = search_path.join(module_name);
            if candidate2.exists() {
                return Ok(candidate2);
            }
        }
        
        Err(RuntimeError::new(format!("Module '{}' not found in search paths", module_name))
            .with_hint("Check that the module file exists and is in one of the search paths. Use TXTCODE_MODULE_PATH environment variable to add search paths.".to_string()))
    }

    fn normalize_path(&self, path: &Path) -> Result<PathBuf, RuntimeError> {
        // Try to canonicalize if path exists, otherwise return as-is
        if path.exists() {
            path.canonicalize()
                .map_err(|e| RuntimeError::new(format!("Failed to resolve module path: {}", e)))
        } else {
            Ok(path.to_path_buf())
        }
    }

    /// Load and parse a module
    pub fn load_module(&mut self, module_path: &Path) -> Result<crate::parser::ast::Program, RuntimeError> {
        let path_str = module_path.to_string_lossy().to_string();
        
        // Check if already loaded
        if let Some(program) = self.loaded_modules.get(&path_str) {
            return Ok(program.clone());
        }
        
        // Read and parse module
        let source = fs::read_to_string(module_path)
            .map_err(|e| RuntimeError::new(format!("Failed to read module '{}': {}", path_str, e)))?;
        
        // Extract module metadata (version, feature flags)
        use crate::runtime::module_metadata::ModuleMetadata;
        let metadata = ModuleMetadata::from_source(&source);
        
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()
            .map_err(|e| RuntimeError::new(format!("Failed to tokenize module '{}': {}", path_str, e)))?;
        
        let mut parser = Parser::new(tokens);
        let mut program = parser.parse()
            .map_err(|e| RuntimeError::new(format!("Failed to parse module '{}': {}", path_str, e)))?;
        
        // Apply compatibility migration if needed
        if let Some(source_version) = metadata.get_version() {
            use crate::runtime::compatibility::CompatibilityLayer;
            let compatibility = CompatibilityLayer::new();
            
            // Migrate AST if version is older
            match compatibility.current_version().is_compatible_with(source_version) {
                crate::runtime::compatibility::CompatibilityResult::BackwardCompatible => {
                    // Migration needed - apply it
                    program = compatibility.migrate_ast(program, Some(source_version.clone()))
                        .map_err(|e| RuntimeError::new(format!(
                            "Failed to migrate module '{}' from {}: {}", 
                            path_str, source_version.to_string(), e
                        )))?;
                }
                crate::runtime::compatibility::CompatibilityResult::Incompatible { reason } => {
                    return Err(RuntimeError::new(format!(
                        "Module '{}' version {} is incompatible with runtime: {}",
                        path_str, source_version.to_string(), reason
                    )));
                }
                crate::runtime::compatibility::CompatibilityResult::FullyCompatible => {
                    // No migration needed
                }
            }
        }
        
        // Cache the loaded module
        let program_clone = program.clone();
        self.loaded_modules.insert(path_str, program_clone.clone());
        
        // Return cloned program
        Ok(program_clone)
    }

    /// Check for circular imports
    pub fn check_circular_import(&self, module_path: &Path, import_stack: &[PathBuf]) -> Result<(), RuntimeError> {
        let path_buf = module_path.to_path_buf();
        if import_stack.contains(&path_buf) {
            let cycle: Vec<String> = import_stack.iter()
                .chain(std::iter::once(&path_buf))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            return Err(RuntimeError::new(format!("Circular import detected: {}", cycle.join(" -> ")))
                .with_hint("Remove the circular dependency between modules.".to_string()));
        }
        Ok(())
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

