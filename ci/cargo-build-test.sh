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

# Build/test all BPF programs
cargo +"$rust_stable" test-bpf -- --nocapture
rm -rf target/debug # Prevents running out of space on github action runners

# Build/test all host crates
cargo +"$rust_stable" build
cargo +"$rust_stable" test -- --nocapture

# Run test-client sanity check
cargo +"$rust_stable" run --manifest-path=utils/test-client/Cargo.toml

# client_ristretto isn't in the workspace, test it explictly
# client_ristretto disabled because it requires RpcBanksService, which is no longer supported.
#cargo +"$rust_stable" test --manifest-path=themis/client_ristretto/Cargo.toml -- --nocapture

SWAP_PROGRAM_OWNER_FEE_ADDRESS="HfoTxFR1Tm6kGmWgYWD6J7YHVy1UwqSULUGVLXkJqaKN" \
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
