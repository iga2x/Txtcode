#pragma once

#include <string>
#include <unordered_map>
#include "txtcode/lexer/token.h"

namespace txtcode {

class KeywordMap {
public:
    static TokenKind getKeyword(const std::string& word) {
        static const std::unordered_map<std::string, TokenKind> keywords = {
            {"store", TokenKind::Store},
            {"define", TokenKind::Define},
            {"return", TokenKind::Return},
            {"if", TokenKind::If},
            {"else", TokenKind::Else},
            {"elseif", TokenKind::Elseif},
            {"end", TokenKind::End},
            {"while", TokenKind::While},
            {"for", TokenKind::For},
            {"in", TokenKind::In},
            {"repeat", TokenKind::Repeat},
            {"times", TokenKind::Times},
            {"match", TokenKind::Match},
            {"case", TokenKind::Case},
            {"break", TokenKind::Break},
            {"continue", TokenKind::Continue},
            {"try", TokenKind::Try},
            {"catch", TokenKind::Catch},
            {"import", TokenKind::Import},
            {"from", TokenKind::From},
            {"as", TokenKind::As},
            {"and", TokenKind::And},
            {"or", TokenKind::Or},
            {"not", TokenKind::Not},
        };
        
        auto it = keywords.find(word);
        if (it != keywords.end()) {
            return it->second;
        }
        return TokenKind::Identifier;
    }
    
    static bool isKeyword(const std::string& word) {
        return getKeyword(word) != TokenKind::Identifier;
    }
};

} // namespace txtcode

