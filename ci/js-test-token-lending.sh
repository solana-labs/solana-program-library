#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")"

(cd ../token/js && npm install)

cd ../token-lending/js
npm install
npm run lint
npm run build
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
npm run localnet:down