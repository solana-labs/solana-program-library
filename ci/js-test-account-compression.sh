#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install
cargo install solana-verify

set -x
solana-verify build --library-name spl_account_compression && solana-verify build --library-name spl_noop
cd account-compression/sdk
pnpm install
pnpm build
pnpm lint
pnpm test
