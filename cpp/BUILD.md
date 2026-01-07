# Building the C++ Implementation

## Quick Start

```bash
cd cpp
mkdir build && cd build
cmake ..
make
./txtcode_cpp run ../../examples/hello.txt
```

## Requirements

- **CMake**: 3.15 or later
- **Compiler**: C++17 compatible
  - GCC 7+ (Linux)
  - Clang 5+ (macOS/Linux)
  - MSVC 2017+ (Windows)

## Build Options

```bash
# Debug build
cmake -DCMAKE_BUILD_TYPE=Debug ..

# Release build (default)
cmake -DCMAKE_BUILD_TYPE=Release ..

# With tests
cmake -DBUILD_TESTS=ON ..

# Without examples
cmake -DBUILD_EXAMPLES=OFF ..
```

## Troubleshooting

### Compilation Errors

If you encounter compilation errors:

1. **Check C++17 support**: Ensure your compiler supports C++17
   ```bash
   g++ --version  # Should be 7+
   ```

2. **CMake version**: Update CMake if needed
   ```bash
   cmake --version  # Should be 3.15+
   ```

3. **Clean build**: Try a clean build
   ```bash
   rm -rf build
   mkdir build && cd build
   cmake ..
   make
   ```

### Missing Dependencies

Currently, the C++ implementation has minimal dependencies. Future features may require:
- OpenSSL (for cryptographic functions)
- LLVM (for native code generation)
- Other libraries as needed

## Development

### Adding New Source Files

1. Add `.cpp` file to `SOURCES` in `CMakeLists.txt`
2. Add `.h` file to `HEADERS` in `CMakeLists.txt`
3. Rebuild:
   ```bash
   cd build
   cmake ..
   make
   ```

### Code Style

- Use C++17 features (smart pointers, variants, etc.)
- Follow RAII principles
- Prefer `const` correctness
- Use namespaces (`txtcode::`)
- Header guards: `#pragma once`

