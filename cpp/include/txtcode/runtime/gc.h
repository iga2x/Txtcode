#pragma once

#include "txtcode/runtime/vm.h"
#include <vector>
#include <unordered_map>

namespace txtcode {

/// Simple mark-and-sweep garbage collector
class GarbageCollector {
public:
    GarbageCollector();
    
    // Collect garbage from stack and globals
    void collect(std::vector<Value>& stack, 
                 std::unordered_map<std::string, Value>& globals);
    
    // Mark a value as reachable
    void mark(const Value& value);
    
    // Get collection statistics
    std::size_t getCollectedCount() const { return collected_count_; }
    std::size_t getCollectionCount() const { return collection_count_; }

private:
    std::size_t collected_count_;
    std::size_t collection_count_;
    
    void markValue(const Value& value);
    void sweep(std::vector<Value>& stack,
               std::unordered_map<std::string, Value>& globals);
};

} // namespace txtcode
