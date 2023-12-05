#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
cd account-compression/sdk
pnpm install
pnpm build
pnpm build:program
# pnpm lint # re-enable when lints are fixed
pnpm test
