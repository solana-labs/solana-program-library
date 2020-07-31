#!/usr/bin/env bash

set -e

# Build programs as clients
../do.sh build-lib memo -D warnings
cargo run --manifest-path=client/Cargo.toml
