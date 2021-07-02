#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

npm install --global yarn

set -x
cd token-lending/js
yarn install --pure-lockfile
yarn run lint
yarn run build
