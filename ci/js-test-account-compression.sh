#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
cd account-compression/sdk

yarn install --pure-lockfile
yarn build
yarn test
