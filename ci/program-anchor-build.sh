#!/usr/bin/env bash

set -e

source ci/rust-version.sh stable
echo "installing solana"
source ci/solana-version.sh install
echo "installing anchor"
source ci/anchor-cliversion.sh install

set -x

echo "cargo installing anchor"
anchor build account-compression