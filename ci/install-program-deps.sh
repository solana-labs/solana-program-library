#!/usr/bin/env bash

set -e

source ci/rust-version.sh stable
source ci/solana-version.sh install

set -x

cargo --version
cargo install rustfilt || true

cargo +"$rust_stable" build-sbf --version
