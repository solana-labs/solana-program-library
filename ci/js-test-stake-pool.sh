#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."

pnpm install
pnpm format
pnpm build

cd stake-pool/js
pnpm lint
pnpm test
