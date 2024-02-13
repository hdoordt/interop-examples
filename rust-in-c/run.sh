#! /usr/bin/env bash
set -euxo pipefail

# Build the crate
cargo build

# Compile with dynamic lib
clang target/debug/librust_in_c.so main.c -o rust-in-c-dynamic

# Compile and link statically
clang main.c target/debug/librust_in_c.a -o rust-in-c-static

# Run dynamic!
./rust-in-c-dynamic

# Run static!
./rust-in-c-static
