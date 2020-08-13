#!/usr/bin/env bash

set -ex

# Test program
cd "$(dirname "$0")/.."
./do.sh fmt token-swap --all -- --check
./do.sh clippy token-swap -- --deny=warnings

SPL_CBINDGEN=1 ./do.sh build-lib token-swap -D warnings
git diff --exit-code token-swap/program/inc/token-swap.h
cc token-swap/program/inc/token-swap.h -o target/token-swap.gch

./do.sh build token
./do.sh build token-swap
./do.sh doc token-swap

# TODO: Uncomment once "Undefined symbols for architecture x86_64: _sol_create_program_address" is resolved
#./do.sh test token-swap

# Install dependency project
(
  cd token/js
  npm install
)

# Test js bindings
cd token-swap/js
npm install
npm run lint
npm run flow

# TODO: Uncomment once https://github.com/solana-labs/solana/issues/11465 is resolved
# npm run cluster:localnet
# npm run localnet:down
# npm run localnet:update
# npm run localnet:up
# npm run start
# npm run localnet:down
