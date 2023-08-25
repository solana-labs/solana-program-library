#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd single-pool/js
pnpm install
pnpm run lint
pnpm build
pnpm test
