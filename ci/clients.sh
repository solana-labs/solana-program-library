#!/usr/bin/env bash

set -e

# Build programs as clients
cd "$(dirname "$0")/.."
./do.sh build-lib memo -D warnings
cargo run --manifest-path=ci/client/Cargo.toml
