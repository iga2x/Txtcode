# Txt-code Programming Language - C++ Implementation

This is a C++ port of the Txt-code programming language, providing the same functionality as the Rust implementation but written in C++17.

## Project Structure

```
cpp/
├── CMakeLists.txt          # Build configuration
├── README.md              # This file
├── include/               # Header files
│   └── txtcode/
│       ├── lexer/        # Lexer (tokenizer)
│       ├── parser/       # Parser (AST builder)
│       ├── typecheck/    # Type checking
│       ├── compiler/     # Code generation
│       ├── runtime/      # Virtual machine
│       ├── security/     # Security features
│       ├── stdlib/       # Standard library
│       └── cli/          # Command-line interface
└── src/                  # Source files
    └── (mirrors include structure)
```

## Building

### Prerequisites

- CMake 3.15 or later
- C++17 compatible compiler (GCC 7+, Clang 5+, MSVC 2017+)
- Make or Ninja (optional, for build system)

### Build Instructions

**Build:**
```bash
cd cpp
./build.sh                    # Release build (default)
./build.sh Debug              # Debug build
./build.sh -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTS=ON  # Advanced options
```

**Manual Build:**
```bash
cd cpp
mkdir -p testzone/build
cd testzone/build
cmake ../..
make
```

Or with Ninja:
```bash
cd cpp
mkdir -p testzone/build
cd testzone/build
cmake -G Ninja ../..
ninja
```

### Running

**If using testzone build:**
```bash
# Run a Txt-code program
./testzone/build/txtcode_cpp run ../../examples/hello.txt

# Start REPL
./testzone/build/txtcode_cpp repl

# Compile a program
./testzone/build/txtcode_cpp compile ../../examples/hello.txt
```

**If using standard build:**
```bash
# Run a Txt-code program
./build/txtcode_cpp run ../examples/hello.txt

# Start REPL
./build/txtcode_cpp repl

# Compile a program
./build/txtcode_cpp compile ../examples/hello.txt
```

## Current Implementation Status

### ✅ Completed
- **Lexer**: Full tokenizer implementation
  - Token types (literals, keywords, operators, punctuation)
  - String, number, and identifier scanning
  - Comment handling
  - Arrow operator (→) support

- **Parser**: Basic parser implementation
  - AST node definitions
  - Statement parsing (assignments, functions, control flow)
  - Expression parsing (binary, unary, calls, etc.)
  - Block parsing

- **CLI**: Basic command-line interface
  - `run` command to execute programs
  - `repl` command for interactive shell
  - `compile` command (stub)

### 🚧 In Progress / TODO
- **Runtime/VM**: Virtual machine for bytecode execution
- **Type Checking**: Type inference and validation
- **Compiler**: Bytecode code generation
- **Standard Library**: Core functions and utilities
- **Security Features**: Obfuscation and encryption
- **Optimizer**: Code optimization passes

## Features

### Language Features
- Hybrid syntax: `action → data` or `action data`
- Variables, functions, control flow
- Arrays, maps, and complex data structures
- Pattern matching
- Error handling (try/catch)

### Implementation Features
- Modern C++17 with smart pointers
- Type-safe AST using variants
- Modular architecture
- CMake build system

## Differences from Rust Version

The C++ implementation aims to match the Rust version's functionality, but uses:
- `std::unique_ptr` instead of Rust's ownership system
- `std::variant` for type-safe unions
- `std::vector` and standard containers
- Exception handling instead of Result types
- Manual memory management with smart pointers

## Build Scripts

### Available Scripts

- **`build.sh`** - Build in `testzone/build/` directory
- **`clean.sh`** - Clean all build directories (build/ and testzone/)
- **`install.sh`** - Build and install to system

### Usage

```bash
# Build
./build.sh                    # Release build (default)
./build.sh Debug              # Debug build
./build.sh -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTS=ON  # Advanced cmake options

# Clean
./clean.sh                    # Clean all build directories

# Install
./install.sh                  # Install to /usr/local
./install.sh /custom/path     # Install to custom path
```

## Development

### Adding New Features

1. **Lexer**: Add token types in `include/txtcode/lexer/token.h`
2. **Parser**: Add parsing rules in `src/parser/parser.cpp`
3. **Runtime**: Implement execution in `src/runtime/`
4. **CLI**: Add commands in `src/cli/main.cpp`

### Testing

Tests can be added in a `tests/` directory and integrated with CMake's testing framework:

```cmake
enable_testing()
add_subdirectory(tests)
```

## Example Usage

```cpp
#include "txtcode/lexer/lexer.h"
#include "txtcode/parser/parser.h"

std::string source = R"(
    print → "Hello, World!"
    store → name → "Alice"
)";

txtcode::Lexer lexer(source);
auto tokens = lexer.tokenize();

txtcode::Parser parser(tokens);
auto program = parser.parse();
```

## Contributing

This is a port of the Rust implementation. When adding features:
1. Check the Rust version for reference
2. Maintain API compatibility where possible
3. Follow C++ best practices (RAII, smart pointers, const correctness)
4. Update this README with new features

## License

Same license as the main project (MIT OR Apache-2.0)

