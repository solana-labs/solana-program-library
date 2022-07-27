#!/usr/bin/env bash
set -ex

cd "$(dirname "$0")"

# shellcheck source=ci/env.sh
source ../ci/env.sh

# Publish only if in CI, vercel token is present, and it's not a pull request
if [[ -n $CI ]] && [[ -n $VERCEL_TOKEN ]] && [[ -z $CI_PULL_REQUEST ]]; then
  PUBLISH_DOCS=true
else
  PUBLISH_DOCS=
fi

if [[ -n $PUBLISH_DOCS ]]; then
  npm install --global docusaurus-init
  docusaurus-init
  npm install --global vercel
fi

# Build from /src into /build
npm run build

if [[ -n $PUBLISH_DOCS ]]; then
    ./publish-docs.sh
fi
