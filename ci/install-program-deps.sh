#!/usr/bin/env bash

set -ex

source ci/rust-version.sh stable
source ci/solana-version.sh install

cargo --version
cargo install rustfilt || true

solana --version
cargo +"$rust_stable" build-bpf --version
