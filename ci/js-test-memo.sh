#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
pnpm install
pnpm format

cd memo/js
pnpm lint
pnpm build
pnpm test
