#!/usr/bin/env bash
#
# Runs all program tests and builds a code coverage report
#
set -ex

cd "$(dirname "$0")"

if ! which grcov; then
  echo "Error: grcov not found.  Try |cargo install grcov|"
  exit 1
fi

rm *.profraw || true
rm **/**/*.profraw || true
rm -r target/coverage || true

# run tests with instrumented binary
RUST_LOG="error" CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test --features test-bpf

# generate report
mkdir -p target/coverage/html

grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html

grcov . --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/tests.lcov

# cleanup
rm *.profraw || true
rm **/**/*.profraw || true
