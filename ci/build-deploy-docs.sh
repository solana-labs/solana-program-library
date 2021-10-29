#!/usr/bin/env bash

set -e -x

cwd=$(pwd)

npm install --global yarn

cd ${cwd}/token/js
yarn
yarn run docs

cd ${cwd}/token-lending/js
yarn install --pure-lockfile
yarn run docs

cd ${cwd}/stake-pool/js
yarn
yarn run docs

cd ${cwd}/token-swap/js
yarn
yarn run docs
