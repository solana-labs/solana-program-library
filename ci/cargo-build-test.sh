#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

source ./ci/rust-version.sh stable
source ./ci/solana-version.sh install

export RUSTFLAGS="-D warnings"
export RUSTBACKTRACE=1

set -x

# Build/test all BPF programs
cargo +"$rust_stable" test-bpf -- --nocapture

# Build/test all host crates
cargo +"$rust_stable" build
cargo +"$rust_stable" test -- --nocapture

# Run test-client sanity check
cargo +"$rust_stable" run --manifest-path=utils/test-client/Cargo.toml

# client_ristretto isn't in the workspace, test it explictly
cargo +"$rust_stable" test --manifest-path=themis/client_ristretto/Cargo.toml -- --nocapture

SWAP_PROGRAM_OWNER_FEE_ADDRESS="SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8" \
  cargo +"$rust_stable" build-bpf \
    --manifest-path=token-swap/program/Cargo.toml \
    --features production \
    --bpf-out-dir target/deploy-production
mv target/deploy-production/spl_token_swap.so target/deploy/spl_token_swap_production.so

#  # Check generated C headers
#  cargo run --manifest-path=utils/cgen/Cargo.toml
#
#  git diff --exit-code token/program/inc/token.h
#  cc token/program/inc/token.h -o target/token.gch
#  git diff --exit-code token-swap/program/inc/token-swap.h
#  cc token-swap/program/inc/token-swap.h -o target/token-swap.gch

exit 0
