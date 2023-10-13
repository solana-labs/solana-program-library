#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd single-pool/js
pnpm install

cd packages/modern
pnpm run lint
pnpm build

cd ../classic
pnpm run lint
pnpm build
pnpm test
