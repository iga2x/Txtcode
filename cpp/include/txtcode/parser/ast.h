#pragma once

#include <string>
#include <vector>
#include <memory>
#include <variant>
#include "txtcode/lexer/token.h"

namespace txtcode {

// Forward declarations
struct Type;
struct Expression;
struct Statement;
struct Parameter;
struct MatchCase;
struct Pattern;

// Define all data structures first, before they're used in variants

struct Type {
    enum class TypeKind {
        Int,
        Float,
        String,
        Bool,
        Null,
        Array,
        Map,
        Function,
        Any,
    };
    
    TypeKind kind;
    std::vector<std::unique_ptr<Type>> parameters; // For Array, Map, Function
};

struct Parameter {
    std::string name;
    std::unique_ptr<Type> type_annotation;
    Span span;
};

struct Pattern {
    enum class PatternType {
        Literal,
        Identifier,
        Wildcard,
    };
    
    PatternType type;
    std::variant<
        std::unique_ptr<Expression>,
        std::string,
        std::monostate
    > data;
};

struct MatchCase {
    Pattern pattern;
    std::unique_ptr<Expression> guard;
    std::vector<std::unique_ptr<Statement>> body;
    Span span;
};

// Expression data structures
struct LiteralData {
    std::variant<std::int64_t, double, std::string, bool, std::nullptr_t> value;
};

struct BinaryData {
    TokenKind op;
    std::unique_ptr<Expression> left;
    std::unique_ptr<Expression> right;
};

struct UnaryData {
    TokenKind op;
    std::unique_ptr<Expression> operand;
};

struct CallData {
    std::unique_ptr<Expression> callee;
    std::vector<std::unique_ptr<Expression>> arguments;
};

struct IndexData {
    std::unique_ptr<Expression> object;
    std::unique_ptr<Expression> index;
};

struct MemberData {
    std::unique_ptr<Expression> object;
    std::string member;
};

struct ArrayData {
    std::vector<std::unique_ptr<Expression>> elements;
};

struct MapData {
    std::vector<std::pair<std::unique_ptr<Expression>, 
                          std::unique_ptr<Expression>>> entries;
};

struct LambdaData {
    std::vector<Parameter> params;
    std::vector<std::unique_ptr<Statement>> body;
};

// Statement data structures
struct AssignmentData {
    std::string name;
    std::unique_ptr<Type> type_annotation;
    std::unique_ptr<Expression> value;
};

struct FunctionDefData {
    std::string name;
    std::vector<Parameter> params;
    std::unique_ptr<Type> return_type;
    std::vector<std::unique_ptr<Statement>> body;
};

struct ReturnData {
    std::unique_ptr<Expression> value;
};

struct IfData {
    std::unique_ptr<Expression> condition;
    std::vector<std::unique_ptr<Statement>> then_branch;
    std::vector<std::pair<std::unique_ptr<Expression>, 
                          std::vector<std::unique_ptr<Statement>>>> else_if_branches;
    std::vector<std::unique_ptr<Statement>> else_branch;
};

struct WhileData {
    std::unique_ptr<Expression> condition;
    std::vector<std::unique_ptr<Statement>> body;
};

struct ForData {
    std::string variable;
    std::unique_ptr<Expression> iterable;
    std::vector<std::unique_ptr<Statement>> body;
};

struct RepeatData {
    std::unique_ptr<Expression> count;
    std::vector<std::unique_ptr<Statement>> body;
};

struct MatchData {
    std::unique_ptr<Expression> value;
    std::vector<MatchCase> cases;
    std::vector<std::unique_ptr<Statement>> default_case;
};

struct TryData {
    std::vector<std::unique_ptr<Statement>> body;
    std::pair<std::string, std::vector<std::unique_ptr<Statement>>> catch_block;
};

struct ImportData {
    std::vector<std::string> items;
    std::string from;
    std::string alias;
};

// Now define the main AST structures

enum class ExpressionType {
    Literal,
    Identifier,
    Binary,
    Unary,
    Call,
    Index,
    Member,
    Array,
    Map,
    Lambda,
};

struct Expression {
    ExpressionType type;
    Span span;
    
    std::variant<
        LiteralData,                    // Literal
        std::string,                    // Identifier
        BinaryData,                     // Binary
        UnaryData,                      // Unary
        CallData,                       // Call
        IndexData,                      // Index
        MemberData,                     // Member
        ArrayData,                      // Array
        MapData,                        // Map
        LambdaData                      // Lambda
    > data;
    
    Expression(ExpressionType t, const Span& s) : type(t), span(s) {}
};

enum class StatementType {
    Expression,
    Assignment,
    FunctionDef,
    Return,
    If,
    While,
    For,
    Repeat,
    Match,
    Break,
    Continue,
    Try,
    Import,
};

struct Statement {
    StatementType type;
    Span span;
    
    // Union-like data (using variant for type safety)
    std::variant<
        std::unique_ptr<Expression>,    // Expression
        AssignmentData,                 // Assignment
        FunctionDefData,                // FunctionDef
        ReturnData,                     // Return
        IfData,                         // If
        WhileData,                      // While
        ForData,                        // For
        RepeatData,                     // Repeat
        MatchData,                      // Match
        std::monostate,                 // Break, Continue
        TryData,                        // Try
        ImportData                      // Import
    > data;
    
    Statement(StatementType t, const Span& s) : type(t), span(s) {}
};

struct Program {
    std::vector<std::unique_ptr<Statement>> statements;
    Span span;
};

} // namespace txtcode
