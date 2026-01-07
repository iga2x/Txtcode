cmake_minimum_required(VERSION 3.15)
project(txtcode_cpp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Build options
option(BUILD_TESTS "Build test suite" ON)
option(BUILD_EXAMPLES "Build examples" ON)

# Include directories
include_directories(include)

# Source files
set(SOURCES
    src/lexer/token.cpp
    src/lexer/lexer.cpp
    src/lexer/keywords.cpp
    src/parser/ast.cpp
    src/parser/parser.cpp
    src/typecheck/types.cpp
    src/typecheck/checker.cpp
    src/runtime/memory.cpp
    src/runtime/vm.cpp
    src/runtime/bytecode_vm.cpp
    src/runtime/gc.cpp
    src/compiler/bytecode.cpp
    src/compiler/codegen.cpp
    src/compiler/optimizer.cpp
    src/security/obfuscator.cpp
    src/security/encryptor.cpp
    src/stdlib/core.cpp
    src/stdlib/io.cpp
    src/stdlib/crypto.cpp
)

# Headers
set(HEADERS
    include/txtcode/lexer/token.h
    include/txtcode/lexer/lexer.h
    include/txtcode/lexer/keywords.h
    include/txtcode/parser/ast.h
    include/txtcode/parser/parser.h
    include/txtcode/typecheck/types.h
    include/txtcode/typecheck/checker.h
    include/txtcode/runtime/memory.h
    include/txtcode/runtime/vm.h
    include/txtcode/runtime/bytecode_vm.h
    include/txtcode/runtime/gc.h
    include/txtcode/compiler/bytecode.h
    include/txtcode/compiler/codegen.h
    include/txtcode/compiler/optimizer.h
    include/txtcode/security/obfuscator.h
    include/txtcode/security/encryptor.h
    include/txtcode/stdlib/core.h
    include/txtcode/stdlib/io.h
    include/txtcode/stdlib/crypto.h
    include/txtcode/cli/main.h
)

# Create library
add_library(txtcode_lib STATIC ${SOURCES} ${HEADERS})

# Executable
add_executable(txtcode_cpp src/cli/main.cpp)
target_link_libraries(txtcode_cpp txtcode_lib)

# Exclude main.cpp from library
set_target_properties(txtcode_lib PROPERTIES
    EXCLUDE_FROM_ALL TRUE
)

# Dependencies (if using external libraries)
# find_package(OpenSSL REQUIRED)  # For crypto
# target_link_libraries(txtcode_lib OpenSSL::SSL OpenSSL::Crypto)

# Compiler flags
if(MSVC)
    target_compile_options(txtcode_lib PRIVATE /W4)
else()
    target_compile_options(txtcode_lib PRIVATE -Wall -Wextra -pedantic)
endif()

# Install
install(TARGETS txtcode_cpp DESTINATION bin)
install(DIRECTORY include/ DESTINATION include)

# Custom clean target
add_custom_target(clean-all
    COMMAND ${CMAKE_COMMAND} -E remove_directory ${CMAKE_BINARY_DIR}
    COMMENT "Removing build directory"
)

# Testzone build support
# When building in testzone, adjust paths accordingly
if(CMAKE_SOURCE_DIR MATCHES "testzone")
    # Building from testzone directory
    message(STATUS "Building in testzone mode")
endif()

