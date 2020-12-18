#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

set -x
cd token/js
npm install
npm run lint
npm run flow
npx tsc module.d.ts
npm run start-with-test-validator
PROGRAM_VERSION=2.0.4 npm run start-with-test-validator
