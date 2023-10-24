#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

source ./ci/rust-version.sh stable
source ./ci/solana-version.sh

export RUSTFLAGS="-D warnings"
export RUSTBACKTRACE=1

set -x

# Build all C examples
make -C examples/c

# Build/test all host crates
cargo +"$rust_stable" build --workspace --exclude spl-token-cli --exclude spl-token-upgrade-cli --exclude spl-single-pool-cli --exclude spl-transfer-hook-cli
cargo +"$rust_stable" test --workspace --exclude spl-token-cli --exclude spl-token-upgrade-cli --exclude spl-single-pool-cli --exclude spl-transfer-hook-cli -- --nocapture

# Run test-client sanity check
cargo +"$rust_stable" run --manifest-path=utils/test-client/Cargo.toml

#  # Check generated C headers
#  cargo run --manifest-path=utils/cgen/Cargo.toml
#
#  git diff --exit-code token/program/inc/token.h
#  cc token/program/inc/token.h -o target/token.gch
#  git diff --exit-code token-swap/program/inc/token-swap.h
#  cc token-swap/program/inc/token-swap.h -o target/token-swap.gch

exit 0
