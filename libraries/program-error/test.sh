#!/bin/bash

# Bash script to test the `cargo expand` outputs of each test and compare diffs

rm -rf ./tmp
mkdir ./tmp

cargo clippy

# Omits Rust test config output
cargo expand --test impl | head -n 29 > ./tmp/expand_output_impl_test.rs
cargo expand --test bench | head -n 29 > ./tmp/expand_output_bench_test.rs

diff -u ./tmp/expand_output_impl_test.rs ./tmp/expand_output_bench_test.rs > ./tmp/cargo_expand_diff.txt

if [ -s ./tmp/cargo_expand_diff.txt ]; then
    echo "      ❌ Test failed"
    echo "      ❌ Differences detected in output"
    echo "      ❌ Check the diff file: ./tmp/cargo_expand_diff.txt"
else
    echo "      ✅ Test successful"
    echo "      ✅ No differences found between expanded tests"
fi

