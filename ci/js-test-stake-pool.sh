#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd stake-pool/js
pnpm install
pnpm lint
pnpm build
pnpm test
