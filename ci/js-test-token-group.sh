#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
pnpm install
pnpm format
pnpm build

cd token-group/js
pnpm lint
pnpm test
