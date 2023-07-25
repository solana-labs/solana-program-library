#!/usr/bin/env bash

set -e

source ci/rust-version.sh stable
source ci/solana-version.sh install
source ci/anchor-cliversion.sh install

set -x

cd account-compression && anchor build