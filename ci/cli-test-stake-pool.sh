#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
cd stake-pool/cli
cargo build
./setup-and-test.sh
