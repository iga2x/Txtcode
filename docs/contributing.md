# Contributing to Txt-code

Thank you for your interest in contributing to Txt-code!

## Building from Source

```bash
# Clone the repository
git clone https://github.com/iga2x/txtcode.git
cd txtcode

# Build the project
cargo build --release

# Run tests
cargo test

# Run examples
cargo run -- run examples/hello.tc
```

## Architecture Overview

Txt-code is built in Rust and follows a modular architecture:

- **Lexer**: Tokenizes source code
- **Parser**: Builds Abstract Syntax Tree (AST)
- **Type Checker**: Performs type checking and inference
- **Compiler**: Compiles AST to bytecode (`.txtc`); WASM target available via `--features bytecode`; native backend deferred
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

## Roadmap

Current development status and upcoming work: see [`docs/dev-plan.md`](dev-plan.md).

## Questions?

Feel free to open an issue or contact the maintainers.

