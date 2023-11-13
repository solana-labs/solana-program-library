#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
pnpm install
pnpm build

cd token-lending/js
pnpm lint
