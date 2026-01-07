#include "txtcode/runtime/bytecode_vm.h"
#include <stdexcept>
#include <cmath>
#include <iostream>

namespace txtcode {

BytecodeVM::BytecodeVM() : pc_(0), gc_() {
    stack_.reserve(1024);
}

void BytecodeVM::load(const BytecodeProgram& program) {
    instructions_ = program.instructions;
    functions_ = program.functions;
    pc_ = 0;
    stack_.clear();
    globals_.clear();
    call_stack_.clear();
}

Value BytecodeVM::execute() {
    while (pc_ < instructions_.size()) {
        const Bytecode& instruction = instructions_[pc_];
        
        ExecutionResult result = executeInstruction(instruction);
        
        switch (result.type) {
            case ExecutionResultType::Continue:
                pc_++;
                break;
            case ExecutionResultType::Jump:
                pc_ = result.jump_address;
                break;
            case ExecutionResultType::Return:
                return result.return_value;
            case ExecutionResultType::Halt:
                goto done;
        }
        
        // Periodic garbage collection
        if (pc_ % 100 == 0) {
            gc_.collect(stack_, globals_);
        }
    }
    
done:
    // Return top of stack or null
    if (stack_.empty()) {
        return Value();
    }
    return stack_.back();
}

ExecutionResult BytecodeVM::executeInstruction(const Bytecode& instruction) {
    try {
        switch (instruction.op) {
            // Stack operations
            case BytecodeOp::PushInt: {
                std::int64_t val = std::get<std::int64_t>(instruction.operand);
                push(Value(val));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::PushFloat: {
                double val = std::get<double>(instruction.operand);
                push(Value(val));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::PushString: {
                std::string val = std::get<std::string>(instruction.operand);
                push(Value(val));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::PushBool: {
                bool val = std::get<bool>(instruction.operand);
                push(Value(val));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::PushNull: {
                push(Value());
                return ExecutionResult::Continue();
            }
            
            // Variable operations
            case BytecodeOp::LoadVar: {
                std::string name = std::get<std::string>(instruction.operand);
                Value value = getVariable(name);
                push(value);
                return ExecutionResult::Continue();
            }
            case BytecodeOp::StoreVar: {
                std::string name = std::get<std::string>(instruction.operand);
                Value value = pop();
                setVariable(name, value);
                return ExecutionResult::Continue();
            }
            
            // Arithmetic operations
            case BytecodeOp::Add: {
                Value right = pop();
                Value left = pop();
                push(addValues(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Subtract: {
                Value right = pop();
                Value left = pop();
                push(subtractValues(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Multiply: {
                Value right = pop();
                Value left = pop();
                push(multiplyValues(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Divide: {
                Value right = pop();
                Value left = pop();
                push(divideValues(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Modulo: {
                Value right = pop();
                Value left = pop();
                push(moduloValues(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Power: {
                Value right = pop();
                Value left = pop();
                push(powerValues(left, right));
                return ExecutionResult::Continue();
            }
            
            // Comparison operations
            case BytecodeOp::Equal: {
                Value right = pop();
                Value left = pop();
                push(compareEqual(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::NotEqual: {
                Value right = pop();
                Value left = pop();
                push(compareNotEqual(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Less: {
                Value right = pop();
                Value left = pop();
                push(compareLess(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Greater: {
                Value right = pop();
                Value left = pop();
                push(compareGreater(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::LessEqual: {
                Value right = pop();
                Value left = pop();
                push(compareLessEqual(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::GreaterEqual: {
                Value right = pop();
                Value left = pop();
                push(compareGreaterEqual(left, right));
                return ExecutionResult::Continue();
            }
            
            // Logical operations
            case BytecodeOp::And: {
                Value right = pop();
                Value left = pop();
                push(logicalAnd(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Or: {
                Value right = pop();
                Value left = pop();
                push(logicalOr(left, right));
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Not: {
                Value value = pop();
                push(logicalNot(value));
                return ExecutionResult::Continue();
            }
            
            // Control flow
            case BytecodeOp::Jump: {
                std::size_t addr = std::get<std::size_t>(instruction.operand);
                return ExecutionResult::Jump(addr);
            }
            case BytecodeOp::JumpIfFalse: {
                std::size_t addr = std::get<std::size_t>(instruction.operand);
                Value condition = pop();
                if (condition.isBoolean() && !condition.asBoolean()) {
                    return ExecutionResult::Jump(addr);
                }
                return ExecutionResult::Continue();
            }
            case BytecodeOp::JumpIfTrue: {
                std::size_t addr = std::get<std::size_t>(instruction.operand);
                Value condition = pop();
                if (condition.isBoolean() && condition.asBoolean()) {
                    return ExecutionResult::Jump(addr);
                }
                return ExecutionResult::Continue();
            }
            
            // Function operations
            case BytecodeOp::Call: {
                std::string name = std::get<std::string>(instruction.operand);
                std::size_t arg_count = std::get<std::size_t>(instruction.operand);
                callFunction(name, arg_count);
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Return: {
                Value return_value = stack_.empty() ? Value() : pop();
                returnFromFunction(return_value);
                return ExecutionResult::Return(return_value);
            }
            
            // Built-in functions
            case BytecodeOp::Print: {
                Value value = pop();
                std::cout << value.toString() << std::endl;
                return ExecutionResult::Continue();
            }
            
            // Stack manipulation
            case BytecodeOp::Pop: {
                pop();
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Dup: {
                if (stack_.empty()) {
                    throw RuntimeError("Stack underflow in Dup");
                }
                push(stack_.back());
                return ExecutionResult::Continue();
            }
            case BytecodeOp::Swap: {
                if (stack_.size() < 2) {
                    throw RuntimeError("Stack underflow in Swap");
                }
                std::swap(stack_[stack_.size() - 1], stack_[stack_.size() - 2]);
                return ExecutionResult::Continue();
            }
            
            case BytecodeOp::Nop:
                return ExecutionResult::Continue();
            
            default:
                throw RuntimeError("Unimplemented instruction");
        }
    } catch (const RuntimeError& e) {
        throw;
    } catch (const std::exception& e) {
        throw RuntimeError("Runtime error: " + std::string(e.what()));
    }
}

void BytecodeVM::push(const Value& value) {
    stack_.push_back(value);
}

Value BytecodeVM::pop() {
    if (stack_.empty()) {
        throw RuntimeError("Stack underflow");
    }
    Value value = stack_.back();
    stack_.pop_back();
    return value;
}

Value BytecodeVM::peek() const {
    if (stack_.empty()) {
        throw RuntimeError("Stack underflow");
    }
    return stack_.back();
}

bool BytecodeVM::stackEmpty() const {
    return stack_.empty();
}

std::size_t BytecodeVM::stackSize() const {
    return stack_.size();
}

Value BytecodeVM::getVariable(const std::string& name) const {
    // Check locals first (from call stack)
    if (!locals_.empty()) {
        for (auto it = locals_.rbegin(); it != locals_.rend(); ++it) {
            auto var_it = it->find(name);
            if (var_it != it->end()) {
                return var_it->second;
            }
        }
    }
    
    // Check globals
    auto it = globals_.find(name);
    if (it != globals_.end()) {
        return it->second;
    }
    
    throw RuntimeError("Undefined variable: " + name);
}

void BytecodeVM::setVariable(const std::string& name, const Value& value) {
    // Check locals first
    if (!locals_.empty()) {
        for (auto it = locals_.rbegin(); it != locals_.rend(); ++it) {
            if (it->find(name) != it->end()) {
                (*it)[name] = value;
                return;
            }
        }
    }
    
    // Set in globals
    globals_[name] = value;
}

Value BytecodeVM::popValue() {
    return pop();
}

void BytecodeVM::callFunction(const std::string& name, std::size_t arg_count) {
    auto it = functions_.find(name);
    if (it == functions_.end()) {
        throw RuntimeError("Undefined function: " + name);
    }
    
    const FunctionInfo& func = it->second;
    if (func.param_count != arg_count) {
        throw RuntimeError("Argument count mismatch for function: " + name);
    }
    
    // Create call frame
    CallFrame frame;
    frame.return_address = pc_ + 1;
    frame.stack_start = stack_.size() - arg_count;
    
    // Extract arguments
    std::vector<Value> args;
    for (std::size_t i = 0; i < arg_count; ++i) {
        args.push_back(pop());
    }
    
    // Create local environment
    std::unordered_map<std::string, Value> locals;
    // TODO: Map parameters to local variables
    locals_.push_back(locals);
    
    // Push call frame
    call_stack_.push_back(frame);
    
    // Jump to function
    pc_ = func.address;
}

void BytecodeVM::returnFromFunction(const Value& return_value) {
    if (call_stack_.empty()) {
        throw RuntimeError("Return outside of function");
    }
    
    CallFrame frame = call_stack_.back();
    call_stack_.pop_back();
    
    // Restore stack
    while (stack_.size() > frame.stack_start) {
        stack_.pop_back();
    }
    
    // Pop local environment
    if (!locals_.empty()) {
        locals_.pop_back();
    }
    
    // Push return value
    push(return_value);
    
    // Return to caller
    pc_ = frame.return_address;
}

// Arithmetic operations
Value BytecodeVM::addValues(const Value& left, const Value& right) const {
    if (left.isInteger() && right.isInteger()) {
        return Value(left.asInteger() + right.asInteger());
    }
    if (left.isFloat() || right.isFloat()) {
        double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
        double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
        return Value(l + r);
    }
    if (left.isString() || right.isString()) {
        return Value(left.toString() + right.toString());
    }
    throw RuntimeError("Invalid operands for addition");
}

Value BytecodeVM::subtractValues(const Value& left, const Value& right) const {
    if (left.isInteger() && right.isInteger()) {
        return Value(left.asInteger() - right.asInteger());
    }
    double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
    double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
    return Value(l - r);
}

Value BytecodeVM::multiplyValues(const Value& left, const Value& right) const {
    if (left.isInteger() && right.isInteger()) {
        return Value(left.asInteger() * right.asInteger());
    }
    double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
    double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
    return Value(l * r);
}

Value BytecodeVM::divideValues(const Value& left, const Value& right) const {
    double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
    double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
    if (r == 0.0) {
        throw RuntimeError("Division by zero");
    }
    return Value(l / r);
}

Value BytecodeVM::moduloValues(const Value& left, const Value& right) const {
    if (left.isInteger() && right.isInteger()) {
        if (right.asInteger() == 0) {
            throw RuntimeError("Modulo by zero");
        }
        return Value(left.asInteger() % right.asInteger());
    }
    throw RuntimeError("Modulo requires integer operands");
}

Value BytecodeVM::powerValues(const Value& left, const Value& right) const {
    double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
    double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
    return Value(std::pow(l, r));
}

// Comparison operations
Value BytecodeVM::compareEqual(const Value& left, const Value& right) const {
    if (left.type() != right.type()) {
        return Value(false);
    }
    if (left.isInteger()) return Value(left.asInteger() == right.asInteger());
    if (left.isFloat()) return Value(left.asFloat() == right.asFloat());
    if (left.isString()) return Value(left.asString() == right.asString());
    if (left.isBoolean()) return Value(left.asBoolean() == right.asBoolean());
    if (left.isNull()) return Value(true);
    return Value(false);
}

Value BytecodeVM::compareNotEqual(const Value& left, const Value& right) const {
    Value eq = compareEqual(left, right);
    return Value(!eq.asBoolean());
}

Value BytecodeVM::compareLess(const Value& left, const Value& right) const {
    if (left.isInteger() && right.isInteger()) {
        return Value(left.asInteger() < right.asInteger());
    }
    double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
    double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
    return Value(l < r);
}

Value BytecodeVM::compareGreater(const Value& left, const Value& right) const {
    if (left.isInteger() && right.isInteger()) {
        return Value(left.asInteger() > right.asInteger());
    }
    double l = left.isFloat() ? left.asFloat() : static_cast<double>(left.asInteger());
    double r = right.isFloat() ? right.asFloat() : static_cast<double>(right.asInteger());
    return Value(l > r);
}

Value BytecodeVM::compareLessEqual(const Value& left, const Value& right) const {
    Value less = compareLess(left, right);
    Value eq = compareEqual(left, right);
    return Value(less.asBoolean() || eq.asBoolean());
}

Value BytecodeVM::compareGreaterEqual(const Value& left, const Value& right) const {
    Value greater = compareGreater(left, right);
    Value eq = compareEqual(left, right);
    return Value(greater.asBoolean() || eq.asBoolean());
}

// Logical operations
Value BytecodeVM::logicalAnd(const Value& left, const Value& right) const {
    bool l = left.isBoolean() ? left.asBoolean() : !left.isNull();
    bool r = right.isBoolean() ? right.asBoolean() : !right.isNull();
    return Value(l && r);
}

Value BytecodeVM::logicalOr(const Value& left, const Value& right) const {
    bool l = left.isBoolean() ? left.asBoolean() : !left.isNull();
    bool r = right.isBoolean() ? right.asBoolean() : !right.isNull();
    return Value(l || r);
}

Value BytecodeVM::logicalNot(const Value& value) const {
    bool b = value.isBoolean() ? value.asBoolean() : !value.isNull();
    return Value(!b);
}

} // namespace txtcode
