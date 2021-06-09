#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

source ./ci/rust-version.sh stable
source ./ci/solana-version.sh

export RUSTFLAGS="-D warnings"
export RUSTBACKTRACE=1

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

set -x

cd $program_directory/program

# Build/test BPF program
cargo +"$rust_stable" test-bpf -- --nocapture
