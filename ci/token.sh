#!/usr/bin/env bash

set -ex

# Test program
cd "$(dirname "$0")/.."
./do.sh fmt token --all -- --check
./do.sh clippy token -- --deny=warnings

SPL_CBINDGEN=1 ./do.sh build-lib token -D warnings
git diff --exit-code token/inc/token.h

./do.sh build token
./do.sh doc token
./do.sh test token
cc token/inc/token.h -o target/token.gch

# Test cli
./do.sh fmt token-cli --all -- --check
./do.sh clippy token-cli -- --deny=warnings

# Test js bindings
cd "$(dirname "$0")/../token/js"
npm install
npm run lint
npm run flow
tsc module.d.ts
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
npm run localnet:down
