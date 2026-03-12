use std::collections::HashMap;

/// Intent declaration for a function or module
#[derive(Debug, Clone, PartialEq)]
pub struct IntentDeclaration {
    pub intent: String,
    pub ai_hint: Option<String>,
    pub allowed_actions: Vec<String>,
    pub forbidden_actions: Vec<String>,
}

impl IntentDeclaration {
    pub fn new(intent: String) -> Self {
        Self {
            intent,
            ai_hint: None,
            allowed_actions: Vec::new(),
            forbidden_actions: Vec::new(),
        }
    }

    pub fn with_ai_hint(mut self, hint: String) -> Self {
        self.ai_hint = Some(hint);
        self
    }

    pub fn with_allowed_actions(mut self, actions: Vec<String>) -> Self {
        self.allowed_actions = actions;
        self
    }

    pub fn with_forbidden_actions(mut self, actions: Vec<String>) -> Self {
        self.forbidden_actions = actions;
        self
    }
}

/// Intent violation error
#[derive(Debug, Clone)]
pub enum IntentViolationError {
    Violation {
        function: String,
        intent: String,
        action: String,
        resource: String,
        reason: String,
    },
}

impl std::fmt::Display for IntentViolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IntentViolationError::Violation {
                function,
                intent,
                action,
                resource,
                reason,
            } => {
                write!(f, "IntentViolationError: {} (function: {}, intent: '{}', action: '{}', resource: '{}')",
                    reason, function, intent, action, resource)
            }
        }
    }
}

impl std::error::Error for IntentViolationError {}

/// Intent checker - enforces intent declarations at runtime
pub struct IntentChecker {
    function_intents: HashMap<String, IntentDeclaration>,
    module_intent: Option<IntentDeclaration>,
}

impl IntentChecker {
    pub fn new() -> Self {
        Self {
            function_intents: HashMap::new(),
            module_intent: None,
        }
    }

    /// Register intent declaration for a function
    pub fn register_function_intent(&mut self, name: String, declaration: IntentDeclaration) {
        use crate::tools::logger::log_debug;
        log_debug(&format!(
            "Registering intent for function '{}': '{}'",
            name, declaration.intent
        ));
        self.function_intents.insert(name, declaration);
    }

    /// Set module-level intent
    pub fn set_module_intent(&mut self, declaration: IntentDeclaration) {
        use crate::tools::logger::log_debug;
        log_debug(&format!("Setting module intent: '{}'", declaration.intent));
        self.module_intent = Some(declaration);
    }

    /// Check if an action violates declared intent
    pub fn check_action(
        &self,
        function_name: &str,
        action: &str,   // e.g., "fs.write"
        resource: &str, // e.g., "/tmp/file.txt"
    ) -> Result<(), IntentViolationError> {
        // 1. Get function intent (if exists), otherwise use module intent
        let intent_decl = self
            .function_intents
            .get(function_name)
            .or(self.module_intent.as_ref());

        if let Some(intent_decl) = intent_decl {
            // 2. Check forbidden actions first (most restrictive)
            if intent_decl
                .forbidden_actions
                .iter()
                .any(|a| action.starts_with(a) || a == "*")
            {
                return Err(IntentViolationError::Violation {
                    function: function_name.to_string(),
                    intent: intent_decl.intent.clone(),
                    action: action.to_string(),
                    resource: resource.to_string(),
                    reason: format!(
                        "Action '{}' violates declared intent '{}' (explicitly forbidden)",
                        action, intent_decl.intent
                    ),
                });
            }

            // 3. Check allowed actions (if list is non-empty, only these are allowed)
            if !intent_decl.allowed_actions.is_empty()
                && !intent_decl
                    .allowed_actions
                    .iter()
                    .any(|a| action.starts_with(a) || a == "*")
            {
                return Err(IntentViolationError::Violation {
                    function: function_name.to_string(),
                    intent: intent_decl.intent.clone(),
                    action: action.to_string(),
                    resource: resource.to_string(),
                    reason: format!(
                        "Action '{}' not in allowed actions for intent '{}' (allowed: {:?})",
                        action, intent_decl.intent, intent_decl.allowed_actions
                    ),
                });
            }
        }

        // Intent check passed (or no intent declared)
        Ok(())
    }

    /// Get intent declaration for a function
    pub fn get_function_intent(&self, function_name: &str) -> Option<&IntentDeclaration> {
        self.function_intents.get(function_name)
    }

    /// Get module intent
    pub fn get_module_intent(&self) -> Option<&IntentDeclaration> {
        self.module_intent.as_ref()
    }

    /// Check if function has intent declared
    pub fn has_intent(&self, function_name: &str) -> bool {
        self.function_intents.contains_key(function_name) || self.module_intent.is_some()
    }
}

impl Default for IntentChecker {
    fn default() -> Self {
        Self::new()
    }
}
