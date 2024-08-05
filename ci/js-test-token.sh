#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
pnpm install
pnpm format
pnpm build

cd token/js
pnpm build:program
pnpm lint
pnpm test
