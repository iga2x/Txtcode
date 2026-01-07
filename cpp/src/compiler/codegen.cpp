#include "txtcode/compiler/codegen.h"
#include "txtcode/parser/ast.h"
#include <sstream>

namespace txtcode {

BytecodeCompiler::BytecodeCompiler() 
    : label_counter_(0) {}

BytecodeProgram BytecodeCompiler::compile(const Program& program) {
    instructions_.clear();
    constants_.clear();
    functions_.clear();
    label_counter_ = 0;
    labels_.clear();
    patch_list_.clear();
    
    // First pass: collect function definitions
    for (const auto& stmt : program.statements) {
        if (stmt->type == StatementType::FunctionDef) {
            const auto& data = std::get<FunctionDefData>(stmt->data);
            std::string label = "func_" + data.name;
            labels_[label] = instructions_.size();
            functions_[data.name] = FunctionInfo{
                instructions_.size(),
                data.params.size(),
                0  // local_count
            };
        }
    }
    
    // Second pass: compile statements
    for (const auto& stmt : program.statements) {
        compileStatement(*stmt);
    }
    
    // Patch jump addresses
    patchJumps();
    
    BytecodeProgram result;
    result.instructions = instructions_;
    result.constants = constants_;
    result.functions = functions_;
    return result;
}

void BytecodeCompiler::compileStatement(const Statement& statement) {
    switch (statement.type) {
        case StatementType::Expression: {
            const auto& expr = std::get<std::unique_ptr<Expression>>(statement.data);
            compileExpression(*expr);
            emit(Bytecode(BytecodeOp::Pop));
            break;
        }
        case StatementType::Assignment: {
            const auto& data = std::get<AssignmentData>(statement.data);
            compileExpression(*data.value);
            emit(Bytecode(BytecodeOp::StoreVar, data.name));
            break;
        }
        case StatementType::FunctionDef: {
            // Function definition already collected in first pass
            // In a full implementation, we'd compile the function body here
            break;
        }
        case StatementType::Return: {
            const auto& data = std::get<ReturnData>(statement.data);
            if (data.value) {
                compileExpression(*data.value);
            } else {
                emit(Bytecode(BytecodeOp::PushNull));
            }
            emit(Bytecode(BytecodeOp::Return));
            break;
        }
        case StatementType::If: {
            const auto& data = std::get<IfData>(statement.data);
            compileExpression(*data.condition);
            
            std::string else_label = newLabel();
            std::string end_label = newLabel();
            
            // Jump to else if condition is false
            emitJump(Bytecode(BytecodeOp::JumpIfFalse, std::size_t(0)), else_label);
            
            // Compile then branch
            for (const auto& stmt : data.then_branch) {
                compileStatement(*stmt);
            }
            
            // Jump to end after then branch
            emitJump(Bytecode(BytecodeOp::Jump, std::size_t(0)), end_label);
            
            // Emit else label
            emitLabel(else_label);
            
            // Compile else branch
            for (const auto& stmt : data.else_branch) {
                compileStatement(*stmt);
            }
            
            // Emit end label
            emitLabel(end_label);
            break;
        }
        case StatementType::While: {
            const auto& data = std::get<WhileData>(statement.data);
            std::string loop_label = newLabel();
            std::string end_label = newLabel();
            
            // Emit loop start
            emitLabel(loop_label);
            
            // Compile condition
            compileExpression(*data.condition);
            emitJump(Bytecode(BytecodeOp::JumpIfFalse, std::size_t(0)), end_label);
            
            // Compile body
            for (const auto& stmt : data.body) {
                compileStatement(*stmt);
            }
            
            // Jump back to loop start
            emitJump(Bytecode(BytecodeOp::Jump, std::size_t(0)), loop_label);
            
            // Emit end label
            emitLabel(end_label);
            break;
        }
        case StatementType::For:
        case StatementType::Repeat:
        case StatementType::Match:
        case StatementType::Break:
        case StatementType::Continue:
        case StatementType::Try:
        case StatementType::Import:
            // TODO: Implement these statement types
            break;
    }
}

void BytecodeCompiler::compileExpression(const Expression& expression) {
    switch (expression.type) {
        case ExpressionType::Literal:
            compileLiteral(std::get<LiteralData>(expression.data));
            break;
        case ExpressionType::Identifier: {
            std::string name = std::get<std::string>(expression.data);
            emit(Bytecode(BytecodeOp::LoadVar, name));
            break;
        }
        case ExpressionType::Binary:
            compileBinary(std::get<BinaryData>(expression.data));
            break;
        case ExpressionType::Unary:
            compileUnary(std::get<UnaryData>(expression.data));
            break;
        case ExpressionType::Call:
            compileCall(std::get<CallData>(expression.data));
            break;
        case ExpressionType::Index:
            compileIndex(std::get<IndexData>(expression.data));
            break;
        case ExpressionType::Member:
            compileMember(std::get<MemberData>(expression.data));
            break;
        case ExpressionType::Array:
        case ExpressionType::Map:
        case ExpressionType::Lambda:
            // TODO: Implement these expression types
            break;
    }
}

void BytecodeCompiler::compileBinary(const BinaryData& binary) {
    compileExpression(*binary.left);
    compileExpression(*binary.right);
    
    // Map TokenKind to BytecodeOp
    switch (binary.op) {
        case TokenKind::Plus: emit(Bytecode(BytecodeOp::Add)); break;
        case TokenKind::Minus: emit(Bytecode(BytecodeOp::Subtract)); break;
        case TokenKind::Star: emit(Bytecode(BytecodeOp::Multiply)); break;
        case TokenKind::Slash: emit(Bytecode(BytecodeOp::Divide)); break;
        case TokenKind::Percent: emit(Bytecode(BytecodeOp::Modulo)); break;
        case TokenKind::Power: emit(Bytecode(BytecodeOp::Power)); break;
        case TokenKind::Equal: emit(Bytecode(BytecodeOp::Equal)); break;
        case TokenKind::NotEqual: emit(Bytecode(BytecodeOp::NotEqual)); break;
        case TokenKind::Less: emit(Bytecode(BytecodeOp::Less)); break;
        case TokenKind::Greater: emit(Bytecode(BytecodeOp::Greater)); break;
        case TokenKind::LessEqual: emit(Bytecode(BytecodeOp::LessEqual)); break;
        case TokenKind::GreaterEqual: emit(Bytecode(BytecodeOp::GreaterEqual)); break;
        case TokenKind::And: emit(Bytecode(BytecodeOp::And)); break;
        case TokenKind::Or: emit(Bytecode(BytecodeOp::Or)); break;
        default:
            throw std::runtime_error("Unsupported binary operator");
    }
}

void BytecodeCompiler::compileUnary(const UnaryData& unary) {
    compileExpression(*unary.operand);
    
    switch (unary.op) {
        case TokenKind::Not: emit(Bytecode(BytecodeOp::Not)); break;
        case TokenKind::Minus: {
            // Negation: push 0, then subtract
            emit(Bytecode(BytecodeOp::PushInt, std::int64_t(0)));
            emit(Bytecode(BytecodeOp::Swap));
            emit(Bytecode(BytecodeOp::Subtract));
            break;
        }
        default:
            throw std::runtime_error("Unsupported unary operator");
    }
}

void BytecodeCompiler::compileCall(const CallData& call) {
    // Compile arguments first (reverse order for stack)
    for (auto it = call.arguments.rbegin(); it != call.arguments.rend(); ++it) {
        compileExpression(**it);
    }
    
    // Compile callee
    compileExpression(*call.callee);
    
    // Check if it's a built-in function
    if (call.callee->type == ExpressionType::Identifier) {
        std::string name = std::get<std::string>(call.callee->data);
        if (name == "print") {
            // Built-in print function - pop callee, then print
            emit(Bytecode(BytecodeOp::Pop)); // Remove callee from stack
            emit(Bytecode(BytecodeOp::Print));
            emit(Bytecode(BytecodeOp::PushNull)); // Print returns null
            return;
        }
    }
    
    // Regular function call - simplified for now
    // In a full implementation, we'd need to handle function calls properly
    // For now, just pop the callee since we can't easily pass both name and arg count
    emit(Bytecode(BytecodeOp::Pop)); // Remove callee identifier
    // TODO: Implement proper function call mechanism
    // This requires either:
    // 1. Extending Bytecode to support multiple operands, or
    // 2. Using separate instructions for function name and arg count
    throw std::runtime_error("Function calls not fully implemented yet");
}

void BytecodeCompiler::compileIndex(const IndexData& index) {
    compileExpression(*index.object);
    compileExpression(*index.index);
    emit(Bytecode(BytecodeOp::Index));
}

void BytecodeCompiler::compileMember(const MemberData& member) {
    compileExpression(*member.object);
    emit(Bytecode(BytecodeOp::Member, member.member));
}

void BytecodeCompiler::compileLiteral(const LiteralData& literal) {
    if (std::holds_alternative<std::int64_t>(literal.value)) {
        emit(Bytecode(BytecodeOp::PushInt, std::get<std::int64_t>(literal.value)));
    } else if (std::holds_alternative<double>(literal.value)) {
        emit(Bytecode(BytecodeOp::PushFloat, std::get<double>(literal.value)));
    } else if (std::holds_alternative<std::string>(literal.value)) {
        emit(Bytecode(BytecodeOp::PushString, std::get<std::string>(literal.value)));
    } else if (std::holds_alternative<bool>(literal.value)) {
        emit(Bytecode(BytecodeOp::PushBool, std::get<bool>(literal.value)));
    } else if (std::holds_alternative<std::nullptr_t>(literal.value)) {
        emit(Bytecode(BytecodeOp::PushNull));
    }
}

void BytecodeCompiler::emit(const Bytecode& instruction) {
    instructions_.push_back(instruction);
}

std::string BytecodeCompiler::newLabel() {
    return "label_" + std::to_string(label_counter_++);
}

void BytecodeCompiler::emitLabel(const std::string& label) {
    labels_[label] = instructions_.size();
}

void BytecodeCompiler::emitJump(const Bytecode& jump_instruction, const std::string& label) {
    std::size_t instruction_index = instructions_.size();
    instructions_.push_back(jump_instruction);
    patch_list_.push_back({instruction_index, label});
}

void BytecodeCompiler::patchJumps() {
    for (const auto& [index, label] : patch_list_) {
        auto it = labels_.find(label);
        if (it != labels_.end()) {
            instructions_[index].operand = it->second;
        }
    }
}

} // namespace txtcode
