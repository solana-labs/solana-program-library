#!/usr/bin/env bash

set -ex

if [[ -z $1 ]]; then
  fuzz_targets=(
    token-swap-instructions
  )
else
  fuzz_targets=( "$@" )
fi

for fuzz_target in ${fuzz_targets[@]}; do
  HFUZZ_RUN_ARGS="--run_time 30 --exit_upon_crash" cargo hfuzz run $fuzz_target

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
done
