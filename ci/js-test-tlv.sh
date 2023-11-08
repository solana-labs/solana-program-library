#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
cd libraries/type-length-value/js

pnpm install
pnpm lint
pnpm build
pnpm test
