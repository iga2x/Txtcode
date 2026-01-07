#include "txtcode/runtime/vm.h"
#include "txtcode/parser/ast.h"
#include <sstream>
#include <cmath>

namespace txtcode {

ValueType Value::type() const {
    if (std::holds_alternative<std::int64_t>(data)) return ValueType::Integer;
    if (std::holds_alternative<double>(data)) return ValueType::Float;
    if (std::holds_alternative<std::string>(data)) return ValueType::String;
    if (std::holds_alternative<bool>(data)) return ValueType::Boolean;
    if (std::holds_alternative<std::nullptr_t>(data)) return ValueType::Null;
    if (std::holds_alternative<std::vector<Value>>(data)) return ValueType::Array;
    if (std::holds_alternative<std::unordered_map<std::string, Value>>(data)) return ValueType::Map;
    return ValueType::Null;
}

std::string Value::typeName() const {
    switch (type()) {
        case ValueType::Integer: return "int";
        case ValueType::Float: return "float";
        case ValueType::String: return "string";
        case ValueType::Boolean: return "bool";
        case ValueType::Null: return "null";
        case ValueType::Array: return "array";
        case ValueType::Map: return "map";
        case ValueType::Function: return "function";
    }
    return "unknown";
}

std::string Value::toString() const {
    switch (type()) {
        case ValueType::Integer:
            return std::to_string(asInteger());
        case ValueType::Float: {
            std::ostringstream oss;
            oss << asFloat();
            return oss.str();
        }
        case ValueType::String:
            return asString();
        case ValueType::Boolean:
            return asBoolean() ? "true" : "false";
        case ValueType::Null:
            return "null";
        case ValueType::Array: {
            const auto& arr = asArray();
            std::ostringstream oss;
            oss << "[";
            for (size_t i = 0; i < arr.size(); ++i) {
                if (i > 0) oss << ", ";
                oss << arr[i].toString();
            }
            oss << "]";
            return oss.str();
        }
        case ValueType::Map: {
            const auto& map = asMap();
            std::ostringstream oss;
            oss << "{";
            bool first = true;
            for (const auto& [key, value] : map) {
                if (!first) oss << ", ";
                oss << key << ": " << value.toString();
                first = false;
            }
            oss << "}";
            return oss.str();
        }
        default:
            return "<unknown>";
    }
}

// Environment implementation
Environment::Environment() : parent_(nullptr) {}

Environment::Environment(std::shared_ptr<Environment> parent) : parent_(parent) {}

void Environment::define(const std::string& name, const Value& value) {
    values_[name] = value;
}

Value Environment::get(const std::string& name) const {
    auto it = values_.find(name);
    if (it != values_.end()) {
        return it->second;
    }
    if (parent_) {
        return parent_->get(name);
    }
    throw RuntimeError("Undefined variable: " + name);
}

void Environment::assign(const std::string& name, const Value& value) {
    if (values_.find(name) != values_.end()) {
        values_[name] = value;
        return;
    }
    if (parent_) {
        parent_->assign(name, value);
        return;
    }
    throw RuntimeError("Undefined variable: " + name);
}

bool Environment::has(const std::string& name) const {
    if (values_.find(name) != values_.end()) {
        return true;
    }
    if (parent_) {
        return parent_->has(name);
    }
    return false;
}

// VirtualMachine implementation
VirtualMachine::VirtualMachine() {
    globals_ = std::make_shared<Environment>();
    current_ = globals_;
}

RuntimeError VirtualMachine::createError(const std::string& message) const {
    return RuntimeError(message);
}

Value VirtualMachine::execute(const Program& program) {
    Value result;
    for (const auto& stmt : program.statements) {
        executeStatement(*stmt);
    }
    return Value(); // Return null for now
}

void VirtualMachine::executeStatement(const Statement& stmt) {
    // TODO: Implement statement execution
    // This is a placeholder - full implementation would handle all statement types
}

Value VirtualMachine::evaluateExpression(const Expression& expr) {
    // TODO: Implement expression evaluation
    // This is a placeholder - full implementation would handle all expression types
    return Value();
}

Value VirtualMachine::evaluateBinary(const BinaryData& binary) {
    // TODO: Implement binary expression evaluation
    return Value();
}

Value VirtualMachine::evaluateUnary(const UnaryData& /*unary*/) {
    // TODO: Implement unary expression evaluation
    return Value();
}

Value VirtualMachine::evaluateCall(const CallData& /*call*/) {
    // TODO: Implement function call evaluation
    return Value();
}

Value VirtualMachine::evaluateIndex(const IndexData& /*index*/) {
    // TODO: Implement index evaluation
    return Value();
}

Value VirtualMachine::evaluateMember(const MemberData& /*member*/) {
    // TODO: Implement member access evaluation
    return Value();
}

} // namespace txtcode
