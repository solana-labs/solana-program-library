#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
cd memo/js

pnpm install
pnpm lint
pnpm build
pnpm test
