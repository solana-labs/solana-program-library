#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")"

(cd ../token/js && npm install)

cd ../token-lending/js
sh -c "$(curl -sSfL https://release.solana.com/v1.5.14/install)"
npm install
npm run lint
npm run build
npm run localnet
npm run start
