#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
pnpm install

(cd libraries/type-length-value/js && pnpm build)

cd token-metadata/js
pnpm lint
pnpm build
pnpm test
