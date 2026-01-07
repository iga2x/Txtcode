#!/bin/bash
# Clean testzone directory
cd "$(dirname "$0")"
export CARGO_TARGET_DIR=testzone/target
cargo clean 2>/dev/null || true
rm -rf testzone/target testzone/build testzone/output
echo "Testzone cleaned!"

