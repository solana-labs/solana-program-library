#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
pnpm install

(cd memo/js && pnpm build)

cd token/js
pnpm lint
pnpm build
pnpm test
