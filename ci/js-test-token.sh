#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
cd token/js

yarn install --pure-lockfile
yarn lint
yarn build
yarn test
