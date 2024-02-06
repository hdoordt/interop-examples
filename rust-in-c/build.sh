#! /usr/bin/env bash

cargo build

clang -o rust-in-c target/debug/libcrc_in_rust.so main.c
