use crate::runtime::core::Value;
use std::collections::{HashMap, HashSet};

/// Mark-and-sweep garbage collector
pub struct GarbageCollector {
    allocated_objects: HashSet<*const Value>,
    mark_set: HashSet<*const Value>,
    collection_threshold: usize,
    allocations_since_gc: usize,
}

impl GarbageCollector {
    pub fn new() -> Self {
        Self {
            allocated_objects: HashSet::new(),
            mark_set: HashSet::new(),
            collection_threshold: 1000, // Collect after 1000 allocations
            allocations_since_gc: 0,
        }
    }

    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            allocated_objects: HashSet::new(),
            mark_set: HashSet::new(),
            collection_threshold: threshold,
            allocations_since_gc: 0,
        }
    }

    /// Collect garbage from stack, globals, and scopes
    pub fn collect(
        &mut self,
        stack: &[Value],
        globals: &HashMap<String, Value>,
        scopes: &[HashMap<String, Value>],
    ) {
        self.allocations_since_gc += 1;

        // Only collect if threshold reached
        if self.allocations_since_gc < self.collection_threshold {
            return;
        }

        self.allocations_since_gc = 0;

        // Mark phase
        self.mark_set.clear();

        // Mark all values on stack
        for value in stack.iter() {
            self.mark_value(value);
        }

        // Mark all global variables
        for value in globals.values() {
            self.mark_value(value);
        }

        // Mark all values in local scopes
        for scope in scopes.iter() {
            for value in scope.values() {
                self.mark_value(value);
            }
        }

        // Sweep phase - would free unmarked objects
        // In Rust, this is handled by the borrow checker and drop,
        // but we track for statistics
        let before = self.allocated_objects.len();
        self.allocated_objects
            .retain(|ptr| self.mark_set.contains(ptr));
        let after = self.allocated_objects.len();

        if before > after {
            // Objects were collected
            // In a real implementation, we'd free memory here
        }
    }

    /// Mark a value and all its references
    fn mark_value(&mut self, value: &Value) {
        let ptr = value as *const Value;

        // Avoid cycles
        if self.mark_set.contains(&ptr) {
            return;
        }

        self.mark_set.insert(ptr);
        self.allocated_objects.insert(ptr);

        // Recursively mark nested values
        match value {
            Value::Array(arr) => {
                for elem in arr {
                    self.mark_value(elem);
                }
            }
            Value::Map(map) => {
                for val in map.values() {
                    self.mark_value(val);
                }
            }
            Value::Set(set) => {
                for elem in set {
                    self.mark_value(elem);
                }
            }
            Value::Function(_, _, _, captured_env) => {
                // Mark captured environment in closures
                for val in captured_env.values() {
                    self.mark_value(val);
                }
            }
            Value::Struct(_, fields) => {
                for val in fields.values() {
                    self.mark_value(val);
                }
            }
            Value::Enum(_, _) => {
                // Enum values are simple, no nested values
            }
            _ => {
                // Primitive types don't need marking
            }
        }
    }

    /// Register a new allocation
    pub fn register_allocation(&mut self, value: &Value) {
        let ptr = value as *const Value;
        self.allocated_objects.insert(ptr);
    }

    /// Get GC statistics
    pub fn get_stats(&self) -> GCStats {
        GCStats {
            allocated_objects: self.allocated_objects.len(),
            marked_objects: self.mark_set.len(),
            allocations_since_gc: self.allocations_since_gc,
        }
    }

    /// Force a full garbage collection
    pub fn force_collect(
        &mut self,
        stack: &[Value],
        globals: &HashMap<String, Value>,
        scopes: &[HashMap<String, Value>],
    ) {
        self.allocations_since_gc = self.collection_threshold;
        self.collect(stack, globals, scopes);
    }
}

#[derive(Debug, Clone)]
pub struct GCStats {
    pub allocated_objects: usize,
    pub marked_objects: usize,
    pub allocations_since_gc: usize,
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}
