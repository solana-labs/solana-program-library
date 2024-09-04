#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

pnpm install
pnpm format

cd single-pool/js/packages/modern
pnpm lint
pnpm build

cd ../classic
pnpm build:program
pnpm lint
pnpm build
pnpm test
