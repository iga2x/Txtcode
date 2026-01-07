# Contributing to Txt-code

Thank you for your interest in contributing to Txt-code!

## Building from Source

```bash
# Clone the repository
git clone https://github.com/txtcode/txtcode.git
cd txtcode

# Build the project
cargo build --release

# Run tests
cargo test

# Run examples
cargo run -- run examples/hello.txt
```

## Architecture Overview

Txt-code is built in Rust and follows a modular architecture:

- **Lexer**: Tokenizes source code
- **Parser**: Builds Abstract Syntax Tree (AST)
- **Type Checker**: Performs type checking and inference
- **Compiler**: Generates bytecode, native code, or WebAssembly
- **Runtime**: Executes programs
- **Standard Library**: Core functions and modules

## Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Write tests for new features
- Update documentation

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Submitting Changes

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Update documentation
6. Submit a pull request

## Development Roadmap

See the main plan file for the complete development roadmap. Current priorities:

- Phase 1: Core Language Foundation (✅ Complete)
- Phase 2: Type System and Safety (In Progress)
- Phase 3: Security Features (Planned)
- Phase 4: Compilation and Code Generation (Planned)
- Phase 5: Runtime Environment (Planned)
- Phase 6: Standard Library (Planned)
- Phase 7: Development Tools (Planned)
- Phase 8: Testing and Examples (Planned)
- Phase 9: Documentation and Ecosystem (Planned)

## Questions?

Feel free to open an issue or contact the maintainers.

