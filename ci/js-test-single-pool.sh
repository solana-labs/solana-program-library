#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd single-pool/js
pnpm install

cd packages/modern
pnpm lint
pnpm build

cd ../classic
pnpm build:program
pnpm lint
pnpm build
pnpm test
