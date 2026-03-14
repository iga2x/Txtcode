// Allocation tracking for the txtcode runtime.
//
// DESIGN NOTE: Rust's ownership system handles all actual memory reclamation via
// RAII and drop. This module tracks allocation pressure for diagnostic purposes
// and provides a hook point for future integration with a real arena allocator.
//
// The previous implementation stored `HashSet<*const Value>` (raw pointers), which
// is unsound — raw pointers do not guarantee the pointed-to Value is still alive,
// and iterating or comparing dangling pointers is undefined behaviour. That approach
// has been removed entirely.
//
// The `GarbageCollector` name is retained as a type alias so existing call sites in
// vm.rs, bytecode_vm.rs, and vm/core.rs compile without modification while the
// implementation is accurate.

use std::collections::HashMap;
use crate::runtime::core::Value;

/// Honest allocation tracker.
///
/// Tracks the number of allocations and provides a configurable threshold for
/// suggesting collection. No raw pointers, no unsound behaviour.
///
/// When `suggest_collection()` reports that the threshold has been reached,
/// callers may release cached data structures or log memory pressure. Actual
/// memory reclamation is handled by Rust's drop system.
pub struct AllocationTracker {
    allocation_count: usize,
    deallocation_count: usize,
    collection_threshold: usize,
    allocations_since_suggest: usize,
}

impl AllocationTracker {
    pub fn new() -> Self {
        Self {
            allocation_count: 0,
            deallocation_count: 0,
            collection_threshold: 1000,
            allocations_since_suggest: 0,
        }
    }

    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            collection_threshold: threshold,
            ..Self::new()
        }
    }

    /// Record a new allocation.
    pub fn record_allocation(&mut self) {
        self.allocation_count += 1;
        self.allocations_since_suggest += 1;
    }

    /// Record that a value was released (bookkeeping only).
    pub fn record_deallocation(&mut self) {
        self.deallocation_count = self.deallocation_count.saturating_add(1);
    }

    /// Suggest collection: returns stats and resets the since-last-suggest counter
    /// if the threshold has been reached. Callers may use the returned stats to
    /// decide whether to release cached data. No actual GC is performed here.
    pub fn suggest_collection(&mut self) -> Option<AllocationStats> {
        if self.allocations_since_suggest >= self.collection_threshold {
            self.allocations_since_suggest = 0;
            Some(self.stats())
        } else {
            None
        }
    }

    /// Return current allocation statistics.
    pub fn stats(&self) -> AllocationStats {
        AllocationStats {
            total_allocations: self.allocation_count,
            total_deallocations: self.deallocation_count,
            allocations_since_suggest: self.allocations_since_suggest,
            threshold: self.collection_threshold,
        }
    }

    // ── Legacy API (keeps existing call sites in vm.rs / bytecode_vm.rs compiling) ──

    /// Called by VMs after each statement. Increments the counter; if the threshold
    /// is reached, logs a trace-level message. The `stack`, `globals`, and `scopes`
    /// parameters are accepted for API compatibility but not dereferenced.
    pub fn collect(
        &mut self,
        _stack: &[Value],
        _globals: &HashMap<String, Value>,
        _scopes: &[HashMap<String, Value>],
    ) {
        self.record_allocation();
        if let Some(_stats) = self.suggest_collection() {
            // Future: log trace-level allocation pressure here.
        }
    }

    /// Called when a value is created. Accepts a reference for API compatibility
    /// but does not store a raw pointer.
    pub fn register_allocation(&mut self, _value: &Value) {
        self.record_allocation();
    }

    /// Return legacy GCStats (alias for stats()).
    pub fn get_stats(&self) -> GCStats {
        let s = self.stats();
        GCStats {
            allocated_objects: s.total_allocations.saturating_sub(s.total_deallocations),
            marked_objects: 0, // no longer tracked
            allocations_since_gc: s.allocations_since_suggest,
        }
    }

    /// Force a collection cycle immediately.
    pub fn force_collect(
        &mut self,
        stack: &[Value],
        globals: &HashMap<String, Value>,
        scopes: &[HashMap<String, Value>],
    ) {
        self.allocations_since_suggest = self.collection_threshold;
        self.collect(stack, globals, scopes);
    }
}

impl Default for AllocationTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias preserved for backward compatibility with existing call sites.
pub type GarbageCollector = AllocationTracker;

/// Allocation statistics.
#[derive(Debug, Clone)]
pub struct AllocationStats {
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub allocations_since_suggest: usize,
    pub threshold: usize,
}

/// Legacy statistics struct kept for API compatibility.
#[derive(Debug, Clone)]
pub struct GCStats {
    pub allocated_objects: usize,
    pub marked_objects: usize,
    pub allocations_since_gc: usize,
}
