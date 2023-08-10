#!/usr/bin/env bash

set -e

source ci/rust-version.sh stable
source ci/solana-version.sh install
source ci/install-anchor.sh install

set -x

usage() {
  exitcode=0
  if [[ -n "$1" ]]; then
    exitcode=1
    echo "Error: $*"
  fi
  echo "Usage: $0 [program-directory]"
  exit $exitcode
}

program_directory=$1
if [[ -z $program_directory ]]; then
  usage "No program directory provided"
fi

cd $program_directory
anchor build