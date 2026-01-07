#!/bin/bash
# Build script for testzone

set -e

cd "$(dirname "$0")"
BUILD_DIR="testzone/build"

echo "Building Txt-code C++ implementation..."
mkdir -p ${BUILD_DIR}
cd ${BUILD_DIR}

# Run cmake and make with any additional arguments
if [ $# -eq 0 ]; then
    cmake ../.. -DCMAKE_BUILD_TYPE=Release
    make -j$(nproc)
elif [ $# -eq 1 ] && [ "$1" = "Debug" ] || [ "$1" = "Release" ]; then
    # Simple build type argument
    cmake ../.. -DCMAKE_BUILD_TYPE=${1}
    make -j$(nproc)
else
    # Pass all arguments to cmake (for advanced usage)
    cmake ../.. "$@"
    make -j$(nproc)
fi

echo ""
echo "Build complete!"
echo "Binary: ${BUILD_DIR}/txtcode_cpp"

