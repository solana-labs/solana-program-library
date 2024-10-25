#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."
source ./ci/rust-version.sh stable

cargo_audit_ignores=(
  # ed25519-dalek: Double Public Key Signing Function Oracle Attack
  #
  # Remove once SPL upgrades to ed25519-dalek v2
  --ignore RUSTSEC-2022-0093

  # curve25519-dalek
  #
  # Remove once SPL upgrades to curve25519-dalek v4
  --ignore RUSTSEC-2024-0344

  # Crate:     tonic
  # Version:   0.9.2
  # Title:     Remotely exploitable Denial of Service in Tonic
  # Date:      2024-10-01
  # ID:        RUSTSEC-2024-0376
  # URL:       https://rustsec.org/advisories/RUSTSEC-2024-0376
  # Solution:  Upgrade to >=0.12.3
  --ignore RUSTSEC-2024-0376
)
cargo +"$rust_stable" audit "${cargo_audit_ignores[@]}"
