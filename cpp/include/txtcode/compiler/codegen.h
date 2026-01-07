#pragma once

#include "txtcode/parser/ast.h"
#include "txtcode/compiler/bytecode.h"
#include <unordered_map>
#include <string>

namespace txtcode {

/// Bytecode compiler - converts AST to bytecode
class BytecodeCompiler {
public:
    BytecodeCompiler();
    
    /// Compile AST program to bytecode
    BytecodeProgram compile(const Program& program);
    
private:
    std::vector<Bytecode> instructions_;
    std::vector<Value> constants_;
    std::unordered_map<std::string, FunctionInfo> functions_;
    std::size_t label_counter_;
    std::unordered_map<std::string, std::size_t> labels_;
    std::vector<std::pair<std::size_t, std::string>> patch_list_; // (instruction_index, label_name)
    
    void compileStatement(const Statement& statement);
    void compileExpression(const Expression& expression);
    
    void emit(const Bytecode& instruction);
    std::string newLabel();
    void emitLabel(const std::string& label);
    void emitJump(const Bytecode& jump_instruction, const std::string& label);
    void patchJumps();
    
    // Expression compilation helpers
    void compileBinary(const BinaryData& binary);
    void compileUnary(const UnaryData& unary);
    void compileCall(const CallData& call);
    void compileIndex(const IndexData& index);
    void compileMember(const MemberData& member);
    void compileLiteral(const LiteralData& literal);
};

} // namespace txtcode
