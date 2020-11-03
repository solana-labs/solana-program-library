#!/usr/bin/env bash

set -e

source ci/rust-version.sh stable
source ci/solana-version.sh install

set -x

cargo --version
cargo install rustfilt || true

export PATH="$HOME"/.local/share/solana/install/active_release/bin:"$PATH"
solana --version
cargo +"$rust_stable" build-bpf --version
