#!/usr/bin/env bash

set -ex

# Test program
cd "$(dirname "$0")/.."
./do.sh fmt token --all -- --check
./do.sh clippy token -- --deny=warnings

SPL_CBINDGEN=1 ./do.sh build-lib token -D warnings
git diff --exit-code token/program/inc/token.h
cc token/program/inc/token.h -o target/token.gch

./do.sh build token
./do.sh doc token
./do.sh test token

# Test cli
./do.sh fmt token/cli --all -- --check
./do.sh clippy token/cli -- --deny=warnings

# Test js bindings
cd "$(dirname "$0")/../token/js"
npm install
npm run lint
npm run flow
tsc module.d.ts
npm run cluster:localnet
npm run localnet:down
npm run localnet:update
npm run localnet:up
npm run start
npm run localnet:down
