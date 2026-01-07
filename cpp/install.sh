#!/bin/bash
# Build and install script

set -e

BUILD_DIR="testzone/build"
INSTALL_PREFIX="${1:-/usr/local}"

echo "Building Txt-code C++ implementation..."
mkdir -p ${BUILD_DIR}
cd ${BUILD_DIR}

cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=${INSTALL_PREFIX}
make -j$(nproc)

echo ""
echo "Installing to ${INSTALL_PREFIX}..."
sudo make install

echo ""
echo "Installation complete!"
echo "Binary installed to: ${INSTALL_PREFIX}/bin/txtcode_cpp"

