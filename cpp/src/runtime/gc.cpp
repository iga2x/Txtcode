#include "txtcode/runtime/gc.h"
#include <algorithm>

namespace txtcode {

GarbageCollector::GarbageCollector() 
    : collected_count_(0), collection_count_(0) {}

void GarbageCollector::collect(std::vector<Value>& stack,
                               std::unordered_map<std::string, Value>& globals) {
    collection_count_++;
    
    // Mark phase - mark all reachable values
    for (const auto& value : stack) {
        markValue(value);
    }
    
    for (const auto& [key, value] : globals) {
        markValue(value);
    }
    
    // Sweep phase - remove unmarked values
    sweep(stack, globals);
}

void GarbageCollector::mark(const Value& value) {
    markValue(value);
}

void GarbageCollector::markValue(const Value& /*value*/) {
    // Simple implementation - in a real GC, we'd mark objects
    // For now, we just track that values are reachable
    // This is a placeholder for a more sophisticated GC
}

void GarbageCollector::sweep(std::vector<Value>& /*stack*/,
                             std::unordered_map<std::string, Value>& /*globals*/) {
    // Simple implementation - in a real GC, we'd remove unmarked objects
    // For now, this is a placeholder
    // A full implementation would need to track object lifetimes
}

} // namespace txtcode
