#!/usr/bin/env bash

set -e

cd "$(dirname "$0")/.."

./do.sh update
./do.sh build token
./do.sh build token-swap
./do.sh clippy token-swap
./do.sh doc token-swap
./do.sh test token-swap
cc token-swap/inc/token-swap.h -o token-swap/target/token-swap.gch

cd "token/js"

npm install

cd "../../token-swap/js"

npm install
npm run cluster:devnet
npm run start