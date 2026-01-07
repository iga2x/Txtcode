#include "txtcode/lexer/token.h"
#include <sstream>

namespace txtcode {

std::string Token::toString() const {
    std::ostringstream oss;
    
    switch (kind) {
        case TokenKind::Integer:
            oss << std::get<std::int64_t>(value.value);
            break;
        case TokenKind::Float:
            oss << std::get<double>(value.value);
            break;
        case TokenKind::String:
        case TokenKind::Identifier:
        case TokenKind::Comment:
        case TokenKind::MultiLineComment:
            oss << std::get<std::string>(value.value);
            break;
        case TokenKind::Boolean:
            oss << (std::get<bool>(value.value) ? "true" : "false");
            break;
        case TokenKind::Null:
            oss << "null";
            break;
        case TokenKind::Store:
            oss << "store";
            break;
        case TokenKind::Define:
            oss << "define";
            break;
        case TokenKind::Return:
            oss << "return";
            break;
        case TokenKind::If:
            oss << "if";
            break;
        case TokenKind::Else:
            oss << "else";
            break;
        case TokenKind::End:
            oss << "end";
            break;
        case TokenKind::Arrow:
            oss << "→";
            break;
        case TokenKind::Plus:
            oss << "+";
            break;
        case TokenKind::Minus:
            oss << "-";
            break;
        case TokenKind::Star:
            oss << "*";
            break;
        case TokenKind::Slash:
            oss << "/";
            break;
        case TokenKind::Equal:
            oss << "==";
            break;
        case TokenKind::NotEqual:
            oss << "!=";
            break;
        case TokenKind::Eof:
            oss << "EOF";
            break;
        default:
            oss << "<token>";
            break;
    }
    
    return oss.str();
}

} // namespace txtcode

