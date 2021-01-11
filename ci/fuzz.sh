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

HFUZZ_RUN_ARGS="--run_time $run_time --exit_upon_crash" cargo hfuzz run $fuzz_target

# Until https://github.com/rust-fuzz/honggfuzz-rs/issues/16 is resolved,
# hfuzz does not return an error code on crash, so look for a crash artifact
exit_status=0
for crash_file in ./hfuzz_workspace/"$fuzz_target"/*.fuzz; do
  # Check if the glob gets expanded to existing files.
  if [[ -e "$crash_file" ]]; then
    echo "Error: .fuzz file $crash_file found, reproduce locally with the hexdump:"
    od -t x1 "$crash_file"
    crash_file_base=$(basename $crash_file)
    hex_output_filename=hex_"$crash_file_base"
    echo "Copy / paste this output into a normal file (e.g. $hex_output_filename)"
    echo "Reconstruct the binary file using:"
    echo "xxd -r $hex_output_filename > $crash_file_base"
    echo "To reproduce the problem, run:"
    echo "cargo hfuzz run-debug $fuzz_target $crash_file_base"
    exit_status=1
  fi
done

exit $exit_status
