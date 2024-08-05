#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
pnpm install
pnpm format

cd libraries/type-length-value/js
pnpm lint
pnpm build
pnpm test
