// Memory management interface — intentional no-op stub.
//
// STATUS: STUB — not used by the VM in v0.4.
//
// The AST VM relies on Rust's ownership system and reference-counted Values (Rc/Arc)
// for automatic memory management. GarbageCollector (gc.rs) handles reference-cycle
// collection when enabled.
//
// MemoryManager exists as a hook for a future explicit allocator or custom GC strategy.
// Add the implementation here when needed; the interface is already wired into the
// runtime module re-exports so callers need no changes.

use crate::runtime::core::Value;

/// Memory management hook for the VM. No-op in v0.4; GarbageCollector (gc.rs) is used instead.
pub struct MemoryManager;

impl MemoryManager {
    pub fn new() -> Self {
        Self
    }

    /// Called when a value is heap-allocated. No-op until an explicit allocator is added.
    pub fn allocate(&mut self, _value: Value) {}

    /// Called when a value is freed. No-op until an explicit allocator is added.
    pub fn deallocate(&mut self, _value: &Value) {}
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}
