#!/usr/bin/env bash

cd "$(dirname "$0")/.."

_() {
  echo "travis_fold:start:$1"
  "$@" || exit 1
  echo "travis_fold:end:$1"
}

_ ci/clients.sh
_ ci/memo.sh
_ ci/token.sh
_ ci/token-swap.sh

exit 0
