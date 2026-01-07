#pragma once

#include "txtcode/compiler/bytecode.h"
#include "txtcode/runtime/vm.h"
#include "txtcode/runtime/gc.h"
#include <vector>
#include <unordered_map>
#include <stack>

namespace txtcode {

// Execution result for instruction execution
enum class ExecutionResultType {
    Continue,
    Jump,
    Return,
    Halt,
};

struct ExecutionResult {
    ExecutionResultType type;
    std::size_t jump_address;
    Value return_value;
    
    static ExecutionResult Continue() {
        ExecutionResult result;
        result.type = ExecutionResultType::Continue;
        return result;
    }
    
    static ExecutionResult Jump(std::size_t addr) {
        ExecutionResult result;
        result.type = ExecutionResultType::Jump;
        result.jump_address = addr;
        return result;
    }
    
    static ExecutionResult Return(const Value& val) {
        ExecutionResult result;
        result.type = ExecutionResultType::Return;
        result.return_value = val;
        return result;
    }
    
    static ExecutionResult Halt() {
        ExecutionResult result;
        result.type = ExecutionResultType::Halt;
        return result;
    }
};

struct CallFrame {
    std::size_t return_address;
    std::unordered_map<std::string, Value> local_vars;
    std::size_t stack_start;
};

/// Stack-based bytecode virtual machine
class BytecodeVM {
public:
    BytecodeVM();
    
    // Load and execute a bytecode program
    void load(const BytecodeProgram& program);
    Value execute();
    
    // Execute a single instruction
    ExecutionResult executeInstruction(const Bytecode& instruction);
    
    // Stack operations
    void push(const Value& value);
    Value pop();
    Value peek() const;
    bool stackEmpty() const;
    std::size_t stackSize() const;
    
    // Variable operations
    Value getVariable(const std::string& name) const;
    void setVariable(const std::string& name, const Value& value);
    
    // Value operations
    Value addValues(const Value& left, const Value& right) const;
    Value subtractValues(const Value& left, const Value& right) const;
    Value multiplyValues(const Value& left, const Value& right) const;
    Value divideValues(const Value& left, const Value& right) const;
    Value moduloValues(const Value& left, const Value& right) const;
    Value powerValues(const Value& left, const Value& right) const;
    
    Value compareEqual(const Value& left, const Value& right) const;
    Value compareNotEqual(const Value& left, const Value& right) const;
    Value compareLess(const Value& left, const Value& right) const;
    Value compareGreater(const Value& left, const Value& right) const;
    Value compareLessEqual(const Value& left, const Value& right) const;
    Value compareGreaterEqual(const Value& left, const Value& right) const;
    
    Value logicalAnd(const Value& left, const Value& right) const;
    Value logicalOr(const Value& left, const Value& right) const;
    Value logicalNot(const Value& value) const;

private:
    std::vector<Value> stack_;
    std::unordered_map<std::string, Value> globals_;
    std::vector<std::unordered_map<std::string, Value>> locals_;
    std::unordered_map<std::string, FunctionInfo> functions_;
    std::vector<Bytecode> instructions_;
    std::size_t pc_;  // Program counter
    std::vector<CallFrame> call_stack_;
    GarbageCollector gc_;
    
    Value popValue();
    void callFunction(const std::string& name, std::size_t arg_count);
    void returnFromFunction(const Value& return_value);
};

} // namespace txtcode
