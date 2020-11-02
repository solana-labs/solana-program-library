#!/usr/bin/env bash

set -e

cargo --version
cargo install rustfilt || true

if [[ -n $SOLANA_VERSION ]]; then
  sh -c "$(curl -sSfL https://release.solana.com/$SOLANA_VERSION/install)"
fi

export PATH=/home/runner/.local/share/solana/install/active_release/bin:"$PATH"

solana --version
cargo build-bpf --version
