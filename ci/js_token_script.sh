#!/usr/bin/env bash

set -e

npm install
npm run lint
npm run flow
npx tsc module.d.ts
npm run cluster:localnet
npm run localnet:update
npm run localnet:up
npm run start
PROGRAM_VERSION=2.0.4 npm run start
npm run localnet:down