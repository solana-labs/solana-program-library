#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."

pnpm install

(cd memo/js && pnpm build)
(cd token/js && pnpm build)

cd stake-pool/js
pnpm lint
pnpm build
pnpm test
