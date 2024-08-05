#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

pnpm install
pnpm format
pnpm build

cd token-swap/js
pnpm build:program
pnpm lint
pnpm test
