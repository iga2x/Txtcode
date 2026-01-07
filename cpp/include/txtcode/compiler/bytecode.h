#pragma once

#include <string>
#include <vector>
#include <unordered_map>
#include <cstdint>
#include "txtcode/runtime/vm.h"

namespace txtcode {

// Bytecode instruction set
enum class BytecodeOp {
    // Stack operations
    PushInt,
    PushFloat,
    PushString,
    PushBool,
    PushNull,
    
    // Variable operations
    LoadVar,
    StoreVar,
    
    // Arithmetic operations
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    
    // Comparison operations
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    
    // Logical operations
    And,
    Or,
    Not,
    
    // Bitwise operations
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    BitNot,
    
    // Control flow
    Jump,              // Unconditional jump
    JumpIfFalse,       // Jump if top of stack is false
    JumpIfTrue,        // Jump if top of stack is true
    
    // Function operations
    Call,              // Call function with n arguments
    Return,
    
    // Array/Map operations
    MakeArray,         // Create array with n elements
    MakeMap,           // Create map with n key-value pairs
    Index,             // Index operation
    Member,            // Member access
    
    // Built-in functions
    Print,
    
    // Special
    Pop,               // Pop top of stack
    Dup,               // Duplicate top of stack
    Swap,              // Swap top two stack elements
    Nop,               // No operation
};

struct Bytecode {
    BytecodeOp op;
    std::variant<
        std::int64_t,      // For PushInt, Jump addresses
        double,            // For PushFloat
        std::string,       // For PushString, LoadVar, StoreVar, Call, Member
        bool,              // For PushBool
        std::size_t        // For MakeArray, MakeMap, Call arg count
    > operand;

    Bytecode(BytecodeOp op) : op(op), operand(std::size_t(0)) {}
    Bytecode(BytecodeOp op, std::int64_t val) : op(op), operand(val) {}
    Bytecode(BytecodeOp op, double val) : op(op), operand(val) {}
    Bytecode(BytecodeOp op, const std::string& val) : op(op), operand(val) {}
    Bytecode(BytecodeOp op, bool val) : op(op), operand(val) {}
    Bytecode(BytecodeOp op, std::size_t val) : op(op), operand(val) {}
};

struct FunctionInfo {
    std::size_t address;
    std::size_t param_count;
    std::size_t local_count;
};

struct BytecodeProgram {
    std::vector<Bytecode> instructions;
    std::vector<Value> constants;
    std::unordered_map<std::string, FunctionInfo> functions;
};

} // namespace txtcode
