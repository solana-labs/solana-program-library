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

# The CI build fails with linker errors when there are too many integrations tests in a project
# In order to run the tests we have to split them and run one target at a time 
if [[ $2 = "--split-tests" ]]; then
  split_tests=1
fi

run_test_sbf() {
  if [[ -n $split_tests ]]; then
    # Run unit tests for project
    cargo +"$rust_stable" test --lib
    # Run integration tests one target at a time 
    for test_file in tests/*.rs; do
      test_target="$(basename $test_file .rs)"
      cargo +"$rust_stable" test-sbf --test $test_target -- --nocapture
    done
  else
    set -x
    cargo +"$rust_stable" test-sbf -- --nocapture
  fi
}

if [[ -r $run_dir/Cargo.toml ]]; then
    # Build/test just one BPF program
    cd $run_dir
    run_test_sbf 
    exit 0
fi

run_all=1
for program in $run_dir/program{,-*}; do
  # Build/test all program directories
  if [[ -r $program/Cargo.toml ]]; then
    run_all=
    (
      cd $program
      run_test_sbf
    )
  fi
done

if [[ -n $run_all ]]; then
  # Build/test all directories
  for directory in $(ls -d $run_dir/*/); do
    cd $directory
    run_test_sbf
  done
fi
