#! /usr/bin/env bash
set -euxo pipefail

cargo build

clang -o rust-in-c target/debug/libcrc_in_rust.so main.c

./rust-in-c