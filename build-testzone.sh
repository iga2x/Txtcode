#!/bin/bash
# Build script for testzone
cd "$(dirname "$0")"
mkdir -p testzone/target
export CARGO_TARGET_DIR=testzone/target
cargo "$@"

