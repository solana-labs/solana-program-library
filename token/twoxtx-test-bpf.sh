#!/usr/bin/env bash
#
# Run token-2022 program tests against a Solana v1.12 monorepo that supports
# 2x transactions for testing the SPL Token 2022 Confidential Transfer extension
#

set -e

here="$(dirname "$0")"
cd "$here"

if [[ ! -d twoxtx-solana ]]; then
  echo "twoxtx-solana dir not found"
  exit 1
fi

echo "Build required programs"
./twoxtx-solana/cargo-build-sbf --manifest-path ./program-2022/Cargo.toml
./twoxtx-solana/cargo-build-sbf --manifest-path ../associated-token-account/program/Cargo.toml

echo "Test token-2022"
./twoxtx-solana/cargo-test-sbf --jobs 2 --manifest-path ./program-2022-test/Cargo.toml -- --nocapture

exit 0
