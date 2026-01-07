use crate::runtime::vm::Value;

/// Memory manager for secure memory allocation
pub struct MemoryManager {
    secure_zones: Vec<SecureZone>,
    allocation_count: usize,
    total_allocated: usize,
}

#[derive(Debug, Clone)]
struct SecureZone {
    identifier: String,
    #[allow(dead_code)] // Reserved for future encrypted memory zones
    encrypted: bool,
    size: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            secure_zones: Vec::new(),
            allocation_count: 0,
            total_allocated: 0,
        }
    }

    /// Allocate memory for a value
    pub fn allocate(&mut self, value: &Value) -> usize {
        self.allocation_count += 1;
        let size = self.estimate_size(value);
        self.total_allocated += size;
        size
    }

    /// Create a secure memory zone
    pub fn create_secure_zone(&mut self, identifier: String, encrypted: bool) {
        self.secure_zones.push(SecureZone {
            identifier,
            encrypted,
            size: 0,
        });
    }

    /// Allocate in secure zone
    pub fn allocate_secure(&mut self, zone_id: &str, value: &Value) -> Result<usize, String> {
        let size = self.estimate_size(value);
        if let Some(zone) = self.secure_zones.iter_mut().find(|z| z.identifier == zone_id) {
            zone.size += size;
            self.total_allocated += size;
            Ok(size)
        } else {
            Err(format!("Secure zone '{}' not found", zone_id))
        }
    }

    /// Estimate memory size of a value
    fn estimate_size(&self, value: &Value) -> usize {
        match value {
            Value::Integer(_) => 8,  // i64
            Value::Float(_) => 8,     // f64
            Value::Boolean(_) => 1,
            Value::Null => 0,
            Value::String(s) => s.len() + 8, // String overhead
            Value::Array(arr) => {
                let mut size = 24; // Vec overhead
                for elem in arr {
                    size += self.estimate_size(elem);
                }
                size
            }
            Value::Map(map) => {
                let mut size = 24; // HashMap overhead
                for (k, v) in map {
                    size += k.len() + 8; // Key
                    size += self.estimate_size(v); // Value
                }
                size
            }
            Value::Function { .. } => 64, // Function overhead (simplified)
        }
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            allocation_count: self.allocation_count,
            total_allocated: self.total_allocated,
            secure_zones: self.secure_zones.len(),
        }
    }

    /// Clear all allocations (for testing)
    pub fn clear(&mut self) {
        self.allocation_count = 0;
        self.total_allocated = 0;
        self.secure_zones.clear();
    }

    /// Check for memory leaks (simplified)
    pub fn check_leaks(&self, expected_allocations: usize) -> bool {
        self.allocation_count <= expected_allocations
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub allocation_count: usize,
    pub total_allocated: usize,
    pub secure_zones: usize,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure memory deletion (overwrite sensitive data)
pub fn secure_delete(value: &mut Value) {
    match value {
        Value::String(s) => {
            // Overwrite string data
            s.clear();
            s.push_str(&"0".repeat(s.capacity()));
        }
        Value::Array(arr) => {
            for elem in arr.iter_mut() {
                secure_delete(elem);
            }
            arr.clear();
        }
        Value::Map(map) => {
            for val in map.values_mut() {
                secure_delete(val);
            }
            map.clear();
        }
        _ => {
            // Primitive types don't need special deletion
        }
    }
}
