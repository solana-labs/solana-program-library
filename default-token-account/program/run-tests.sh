#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")"
cargo clippy
cargo build
cargo build-bpf

if [[ $1 = -v ]]; then
  export RUST_LOG=solana=debug
fi

bpf=1 cargo test
# TODO: bpf=0 not supported until native CPI rework in the monorepo completes
#bpf=0 cargo test
