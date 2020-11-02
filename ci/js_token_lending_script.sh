#!/usr/bin/env bash

set -e

(cd ../../token/js && npm install)
npm install
npm run lint
npm run build
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
npm run localnet:down