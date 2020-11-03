#!/usr/bin/env bash

set -ex

cargo --version
cargo install rustfilt || true

export SOLANA_VERSION=v1.4.4
sh -c "$(curl -sSfL https://release.solana.com/$SOLANA_VERSION/install)"
export PATH="$HOME"/.local/share/solana/install/active_release/bin:"$PATH"

solana --version
cargo build-bpf --version
