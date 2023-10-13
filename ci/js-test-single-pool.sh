#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd single-pool/js
pnpm install

cd packages/client
pnpm run lint
pnpm build

cd ../legacy
pnpm run lint
pnpm build
pnpm test
