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

cd $program_directory
run_dir=$(pwd)

if [[ -r $run_dir/Cargo.toml ]]; then
    # Build/test just one BPF program
    set -x
    cd $run_dir
    cargo +"$rust_stable" test-bpf -- --nocapture
    exit 0
fi

run_all=1
for program in $run_dir/program{,-*}; do
  # Build/test all program directories
  if [[ -r $program/Cargo.toml ]]; then
    run_all=
    (
      set -x
      cd $program
      cargo +"$rust_stable" test-bpf -- --nocapture
    )
  fi
done

if [[ -n $run_all ]]; then
  # Build/test all directories
  set -x
  for directory in $(ls -d $run_dir/*/); do
    cd $directory
    cargo +"$rust_stable" test-bpf -- --nocapture
  done
fi
