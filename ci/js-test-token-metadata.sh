#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
pnpm install
pnpm build

cd token-metadata/js
pnpm lint
pnpm test
