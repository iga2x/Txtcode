#!/bin/bash
# Clean build artifacts

cd "$(dirname "$0")"
echo "Cleaning build directories..."
rm -rf build
rm -rf testzone/build
rm -rf testzone
rm -rf cmake-build-*
echo "Build directories cleaned."

