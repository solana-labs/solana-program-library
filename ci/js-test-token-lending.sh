#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd ../token-lending/js
npm install
npm run lint
npm run build
npm run start-with-test-validator
