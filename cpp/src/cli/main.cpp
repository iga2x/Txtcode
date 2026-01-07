#include <iostream>
#include <string>
#include <vector>
#include <fstream>
#include <sstream>
#include "txtcode/lexer/lexer.h"
#include "txtcode/parser/parser.h"
#include "txtcode/compiler/codegen.h"
#include "txtcode/runtime/bytecode_vm.h"

void printUsage(const char* programName) {
    std::cout << "Txt-code Programming Language (C++ Implementation)\n"
              << "Usage: " << programName << " <command> [options]\n\n"
              << "Commands:\n"
              << "  run <file>     Run a Txt-code program\n"
              << "  compile <file> Compile a Txt-code program\n"
              << "  repl           Start interactive REPL\n"
              << "  help           Show this help message\n";
}

std::string readFile(const std::string& filename) {
    std::ifstream file(filename);
    if (!file.is_open()) {
        throw std::runtime_error("Cannot open file: " + filename);
    }
    
    std::stringstream buffer;
    buffer << file.rdbuf();
    return buffer.str();
}

void runFile(const std::string& filename) {
    try {
        std::string source = readFile(filename);
        
        // Lex
        txtcode::Lexer lexer(source);
        std::vector<txtcode::Token> tokens = lexer.tokenize();
        
        std::cout << "Tokens (" << tokens.size() << "):\n";
        for (const auto& token : tokens) {
            std::cout << "  " << token.toString() << "\n";
        }
        
        // Parse
        txtcode::Parser parser(tokens);
        auto program = parser.parse();
        
        std::cout << "\nParsed " << program->statements.size() << " statements\n";
        
        // Compile to bytecode
        txtcode::BytecodeCompiler compiler;
        txtcode::BytecodeProgram bytecode = compiler.compile(*program);
        
        std::cout << "Compiled to " << bytecode.instructions.size() << " bytecode instructions\n";
        
        // Execute bytecode
        txtcode::BytecodeVM vm;
        vm.load(bytecode);
        txtcode::Value result = vm.execute();
        
        std::cout << "Execution complete. Result: " << result.toString() << "\n";
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        std::exit(1);
    }
}

void startRepl() {
    std::cout << "Txt-code REPL (C++ Implementation)\n"
              << "Type 'exit' or 'quit' to exit\n\n";
    
    std::string line;
    while (true) {
        std::cout << "txtcode> ";
        std::getline(std::cin, line);
        
        if (line == "exit" || line == "quit") {
            break;
        }
        
        if (line.empty()) {
            continue;
        }
        
        try {
            txtcode::Lexer lexer(line);
            std::vector<txtcode::Token> tokens = lexer.tokenize();
            
            for (const auto& token : tokens) {
                std::cout << "  " << token.toString() << "\n";
            }
        } catch (const std::exception& e) {
            std::cerr << "Error: " << e.what() << "\n";
        }
    }
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        printUsage(argv[0]);
        return 1;
    }
    
    std::string command = argv[1];
    
    if (command == "help" || command == "--help" || command == "-h") {
        printUsage(argv[0]);
        return 0;
    } else if (command == "run") {
        if (argc < 3) {
            std::cerr << "Error: Expected filename\n";
            return 1;
        }
        runFile(argv[2]);
    } else if (command == "compile") {
        if (argc < 3) {
            std::cerr << "Error: Expected filename\n";
            return 1;
        }
        try {
            std::string source = readFile(argv[2]);
            
            // Lex
            txtcode::Lexer lexer(source);
            std::vector<txtcode::Token> tokens = lexer.tokenize();
            
            // Parse
            txtcode::Parser parser(tokens);
            auto program = parser.parse();
            
            // Compile to bytecode
            txtcode::BytecodeCompiler compiler;
            txtcode::BytecodeProgram bytecode = compiler.compile(*program);
            
            std::cout << "Compiled successfully!\n";
            std::cout << "  Instructions: " << bytecode.instructions.size() << "\n";
            std::cout << "  Functions: " << bytecode.functions.size() << "\n";
            std::cout << "  Constants: " << bytecode.constants.size() << "\n";
            
            // TODO: Save bytecode to file
        } catch (const std::exception& e) {
            std::cerr << "Error: " << e.what() << "\n";
            return 1;
        }
    } else if (command == "repl") {
        startRepl();
    } else {
        std::cerr << "Unknown command: " << command << "\n";
        printUsage(argv[0]);
        return 1;
    }
    
    return 0;
}

