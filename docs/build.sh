#!/usr/bin/env bash
set -ex

cd "$(dirname "$0")"

# shellcheck source=ci/env.sh
source ../ci/env.sh

# Publish only from merge commits and release tags
if [[ -n $CI ]]; then
  if [[ -z $CI_PULL_REQUEST ]]; then
    npm install --global docusaurus-init
    docusaurus-init
    npm install --global vercel
  fi
fi

# Build from /src into /build
npm run build

# Publish only from merge commits and release tags
if [[ -n $CI ]]; then
  if [[ -z $CI_PULL_REQUEST ]]; then
    ./publish-docs.sh
  fi
fi
