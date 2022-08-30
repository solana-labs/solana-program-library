#!/usr/bin/env bash
#
# Patch in a Solana v1.12 monorepo that supports 2x transactions for testing the
# SPL Token 2022 Confidential Transfer extension
#

set -e

here="$(dirname "$0")"
cd "$here"

if [[ ! -d twoxtx-solana ]]; then
  if [[ -n $CI ]]; then
    git config --global user.email "you@example.com"
    git config --global user.name "Your Name"
    git clone https://github.com/solana-labs/solana.git twoxtx-solana
  else
    git clone git@github.com:solana-labs/solana.git twoxtx-solana
  fi
fi

if [[ ! -f twoxtx-solana/.twoxtx-patched ]]; then
  git -C twoxtx-solana am "$PWD"/twoxtx.patch
  touch twoxtx-solana/.twoxtx-patched
fi

../patch.crates-io.sh twoxtx-solana
exit 0
