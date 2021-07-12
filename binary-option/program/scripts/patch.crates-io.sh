#!/usr/bin/env bash
#
# Patches the SPL crates for developing against a local solana monorepo
#

here="$(dirname "$0")"

solana_dir=$1
if [[ -z $solana_dir ]]; then
  echo "Usage: $0 <path-to-solana-monorepo>"
  exit 1
fi

workspace_crates=(
  "$here"/../Cargo.toml
)

if [[ ! -r "$solana_dir"/scripts/read-cargo-variable.sh ]]; then
  echo "$solana_dir is not a path to the solana monorepo"
  exit 1
fi

set -e

solana_dir=$(cd "$solana_dir" && pwd)

source "$solana_dir"/scripts/read-cargo-variable.sh
solana_ver=$(readCargoVariable version "$solana_dir"/sdk/Cargo.toml)

echo "Patching in $solana_ver from $solana_dir"

if ! git diff --quiet && [[ -z $DIRTY_OK ]]; then
  echo "Error: dirty tree"
  exit 1
fi
export DIRTY_OK=1

for crate in "${workspace_crates[@]}"; do
  if grep -q '\[patch.crates-io\]' "$crate"; then
    echo "* $crate is already patched"
  else
    echo "* patched $crate"
    cat >> "$crate" <<PATCH
[patch.crates-io]
solana-clap-utils = {path = "$solana_dir/clap-utils" }
solana-cli-config = {path = "$solana_dir/cli-config" }
solana-client = { path = "$solana_dir/client"}
solana-logger = { path = "$solana_dir/logger"}
solana-program = { path = "$solana_dir/sdk/program" }
solana-program-test = { path = "$solana_dir/program-test" }
solana-remote-wallet = { path = "$solana_dir/remote-wallet"}
solana-sdk = { path = "$solana_dir/sdk" }
solana-validator = { path = "$solana_dir/validator"}
PATCH
  fi
done

"$here"/update-solana-dependencies.sh "$solana_ver"

