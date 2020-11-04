#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")"

(cd ../token/js && npm install)

cd ../token-swap/js
npm install
npm run lint
npm run flow
npx tsc module.d.ts
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
(cd ../../target/bpfel-unknown-unknown/release && mv spl_token_swap_production.so spl_token_swap.so)
SWAP_PROGRAM_OWNER_FEE_ADDRESS="SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8" npm run start
npm run localnet:down