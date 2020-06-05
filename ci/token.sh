#!/usr/bin/env bash

cd "$(dirname "$0")/../token/js"

set -e

npm install
npm run build:program
npm run test
npm run cluster:devnet
npm run start
