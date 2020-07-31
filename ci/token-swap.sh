#!/usr/bin/env bash

set -ex

# Test program
cd "$(dirname "$0")/.."
./do.sh update
./do.sh build token
./do.sh fmt token-swap --all -- --check
./do.sh build-lib token-swap -D warnings
./do.sh build token-swap
./do.sh clippy token-swap -- --deny=warnings
./do.sh doc token-swap
./do.sh test token-swap
cc token-swap/inc/token-swap.h -o token-swap/target/token-swap.gch

# Install dependency project
cd "token/js"
npm install

# Test js bindings
cd "../../token-swap/js"
npm install
npm run lint
npm run flow
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
npm run localnet:down
