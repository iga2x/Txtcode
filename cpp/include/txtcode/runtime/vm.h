#pragma once

#include <string>
#include <vector>
#include <unordered_map>
#include <memory>
#include <variant>
#include <cstdint>

namespace txtcode {

// Runtime value types
enum class ValueType {
    Integer,
    Float,
    String,
    Boolean,
    Null,
    Array,
    Map,
    Function,
};

struct Value {
    std::variant<
        std::int64_t,                           // Integer
        double,                                 // Float
        std::string,                            // String
        bool,                                   // Boolean
        std::nullptr_t,                         // Null
        std::vector<Value>,                     // Array
        std::unordered_map<std::string, Value>  // Map
    > data;

    Value() : data(nullptr) {}
    Value(std::int64_t i) : data(i) {}
    Value(double f) : data(f) {}
    Value(const std::string& s) : data(s) {}
    Value(bool b) : data(b) {}
    Value(std::vector<Value> arr) : data(std::move(arr)) {}
    Value(std::unordered_map<std::string, Value> map) : data(std::move(map)) {}

    ValueType type() const;
    std::string toString() const;
    std::string typeName() const;

    // Type checks
    bool isInteger() const { return std::holds_alternative<std::int64_t>(data); }
    bool isFloat() const { return std::holds_alternative<double>(data); }
    bool isString() const { return std::holds_alternative<std::string>(data); }
    bool isBoolean() const { return std::holds_alternative<bool>(data); }
    bool isNull() const { return std::holds_alternative<std::nullptr_t>(data); }
    bool isArray() const { return std::holds_alternative<std::vector<Value>>(data); }
    bool isMap() const { return std::holds_alternative<std::unordered_map<std::string, Value>>(data); }

    // Getters
    std::int64_t asInteger() const { return std::get<std::int64_t>(data); }
    double asFloat() const { return std::get<double>(data); }
    std::string asString() const { return std::get<std::string>(data); }
    bool asBoolean() const { return std::get<bool>(data); }
    std::vector<Value>& asArray() { return std::get<std::vector<Value>>(data); }
    const std::vector<Value>& asArray() const { return std::get<std::vector<Value>>(data); }
    std::unordered_map<std::string, Value>& asMap() { return std::get<std::unordered_map<std::string, Value>>(data); }
    const std::unordered_map<std::string, Value>& asMap() const { return std::get<std::unordered_map<std::string, Value>>(data); }
};

struct RuntimeError {
    std::string message;
    RuntimeError(const std::string& msg) : message(msg) {}
};

// Environment for variable storage
class Environment {
public:
    Environment();
    Environment(std::shared_ptr<Environment> parent);
    
    void define(const std::string& name, const Value& value);
    Value get(const std::string& name) const;
    void assign(const std::string& name, const Value& value);
    bool has(const std::string& name) const;

private:
    std::unordered_map<std::string, Value> values_;
    std::shared_ptr<Environment> parent_;
};

// Tree-walk interpreter (for direct AST execution)
class VirtualMachine {
public:
    VirtualMachine();
    
    Value execute(const struct Program& program);
    void executeStatement(const struct Statement& stmt);
    Value evaluateExpression(const struct Expression& expr);
    
    RuntimeError createError(const std::string& message) const;

private:
    std::shared_ptr<Environment> globals_;
    std::shared_ptr<Environment> current_;
    
    Value evaluateBinary(const struct BinaryData& binary);
    Value evaluateUnary(const struct UnaryData& unary);
    Value evaluateCall(const struct CallData& call);
    Value evaluateIndex(const struct IndexData& index);
    Value evaluateMember(const struct MemberData& member);
};

} // namespace txtcode
