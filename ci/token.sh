#!/usr/bin/env bash

set -e

(
    cd "$(dirname "$0")/.."

    ./do.sh update
    ./do.sh build token
    ./do.sh doc token
    ./do.sh test token
    cc token/inc/token.h -o token/target/token.gch
)

(
    cd "$(dirname "$0")/../token/js"

    npm install
    npm run cluster:devnet
    npm run start
)
