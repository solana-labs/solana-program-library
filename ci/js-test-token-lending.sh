#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x
pnpm install

(cd memo/js && pnpm build)
(cd token/js && pnpm build)

cd token-lending/js
pnpm lint
pnpm build
