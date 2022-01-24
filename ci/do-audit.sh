#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/rust-version.sh stable

cargo_audit_ignores=(
  # failure is officially deprecated/unmaintained
  #
  # Blocked on multiple upstream crates removing their `failure` dependency.
  --ignore RUSTSEC-2020-0036

  # Potential segfault in the time crate
  #
  # Blocked on chrono and solana_rbpf updating `time` to >= 0.2.23
  --ignore RUSTSEC-2020-0071

  # chrono: Potential segfault in `localtime_r` invocations
  #
  # Blocked due to no safe upgrade
  # https://github.com/chronotope/chrono/issues/499
  --ignore RUSTSEC-2020-0159

  # memmap is officially deprecated/unmaintained, used by honggfuzz
  #
  # Blcoked on honggfuzz, fixed in https://github.com/rust-fuzz/honggfuzz-rs/pull/55
  # need to update honggfuzz dependency whenever the next version is released
  --ignore RUSTSEC-2020-0077
)
cargo +"$rust_stable" audit "${cargo_audit_ignores[@]}"
