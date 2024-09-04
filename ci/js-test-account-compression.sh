#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
pnpm install
pnpm format
cd account-compression/sdk
pnpm build
pnpm build:program
pnpm lint
pnpm test
