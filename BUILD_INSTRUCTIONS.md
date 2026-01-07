# Txt-code Build Instructions

## Quick Start

### 1. Install Rust

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Reload shell environment
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Build Txt-code

```bash
cd /home/iganomono/test/NPL

# Build in debug mode (faster compilation)
cargo build

# Build in release mode (optimized)
cargo build --release
```

### 3. Run Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_lexer
```

### 4. Use Txt-code

```bash
# Run a program (using debug build)
cargo run -- run examples/hello.txt

# Run a program (using release build)
./target/release/txtcode run examples/hello.txt

# Start REPL
cargo run -- repl

# Compile a program
cargo run -- compile examples/hello.txt -o hello.txtc

# Format code
cargo run -- format examples/hello.txt --write

# Lint code
cargo run -- lint examples/hello.txt
```

## Troubleshooting

### Missing Dependencies

If you encounter dependency errors:

```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build
```

### Compilation Errors

If you see compilation errors:

1. Check Rust version: `rustc --version` (should be 1.70+)
2. Update Rust: `rustup update`
3. Clean build: `cargo clean && cargo build`

### Platform-Specific Issues

Some features may have platform-specific requirements:
- **Linux**: Debugger detection uses `/proc/self/status`
- **Windows**: Requires Windows API for some features
- **macOS**: May need additional permissions

## Development Setup

### Recommended IDE

- **VS Code** with Rust Analyzer extension
- **IntelliJ IDEA** with Rust plugin
- **Vim/Neovim** with rust-analyzer LSP

### Useful Commands

```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Build documentation
cargo doc --open

# Run benchmarks (if configured)
cargo bench
```

## Project Dependencies

Key dependencies in `Cargo.toml`:
- **clap** - CLI argument parsing
- **serde** - Serialization
- **aes-gcm** - Encryption
- **sha2** - Hashing
- **tokio** - Async runtime (for networking)
- **rustyline** - REPL line editing

## Next Steps After Building

1. Try the example programs in `examples/`
2. Read the documentation in `docs/`
3. Write your own Txt-code programs
4. Contribute improvements (see `docs/contributing.md`)

