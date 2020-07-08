#!/usr/bin/env bash

set -e

cd "$(dirname "$0")/.."

./do.sh update
./do.sh build token
./do.sh clippy token
./do.sh doc token
./do.sh test token
cc token/inc/token.h -o token/target/token.gch

cd "$(dirname "$0")/../token/js"

npm install
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
npm run localnet:down
