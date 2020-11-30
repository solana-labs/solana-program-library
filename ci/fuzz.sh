#!/usr/bin/env bash

set -e

usage() {
  exitcode=0
  if [[ -n "$1" ]]; then
    exitcode=1
    echo "Error: $*"
  fi
  echo "Usage: $0 [fuzz-target] [run-time-in-seconds]"
  exit $exitcode
}

fuzz_target=$1
if [[ -z $fuzz_target ]]; then
  usage "No fuzz target provided"
fi

run_time=$2
if [[ -z $2 ]]; then
  usage "No runtime provided"
fi

set -x

HFUZZ_RUN_ARGS="--run_time $run_time --exit_upon_crash" cargo hfuzz run $fuzz_target

# Until https://github.com/rust-fuzz/honggfuzz-rs/issues/16 is resolved,
# hfuzz does not return an error code on crash, so look for a crash artifact
for crash_file in ./hfuzz_workspace/"$fuzz_target"/*.fuzz; do
  # Check if the glob gets expanded to existing files.
  if [[ -e "$crash_file" ]]; then
    echo ".fuzz file $crash_file found, meaning some error occurred, exiting"
    exit 1
  fi
  # Break early -- we just need one iteration to see if a failure occurred
  break
done
