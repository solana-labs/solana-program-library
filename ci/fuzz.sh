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
exit_status=0
for crash_file in ./hfuzz_workspace/"$fuzz_target"/*.fuzz; do
  # Check if the glob gets expanded to existing files.
  if [[ -e "$crash_file" ]]; then
    echo ".fuzz file $crash_file found, meaning some error occurred, try to reproduce locall with the contents of the file:"
    cat "$crash_file"
    exit_status=1
  fi
done

exit $exit_status
