#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

pnpm install
pnpm build

cd token-swap/js
pnpm lint
pnpm start-with-test-validator
(cd ../../target/deploy && mv spl_token_swap_production.so spl_token_swap.so)
SWAP_PROGRAM_OWNER_FEE_ADDRESS="HfoTxFR1Tm6kGmWgYWD6J7YHVy1UwqSULUGVLXkJqaKN" pnpm start-with-test-validator
