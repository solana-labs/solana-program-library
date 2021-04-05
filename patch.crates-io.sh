#!/usr/bin/env bash
#
# Patches the SPL crates for developing against a local solana monorepo
#

solana_dir=$1
if [[ -z $solana_dir ]]; then
  echo "Usage: $0 <path-to-solana-monorepo>"
  exit 1
fi

workspace_crates=(
  Cargo.toml
  themis/client_ristretto/Cargo.toml
)

if [[ ! -r "$solana_dir"/scripts/read-cargo-variable.sh ]]; then
  echo "$solana_dir is not a path to the solana monorepo"
  exit 1
fi

set -e

solana_dir=$(cd "$solana_dir" && pwd)
cd "$(dirname "$0")"

source "$solana_dir"/scripts/read-cargo-variable.sh
solana_ver=$(readCargoVariable version "$solana_dir"/sdk/Cargo.toml)

echo "Patching in $solana_ver from $solana_dir"
echo
for crate in "${workspace_crates[@]}"; do
  if grep -q '\[patch.crates-io\]' "$crate"; then
    echo "$crate is already patched"
  else
    cat >> "$crate" <<PATCH
[patch.crates-io]
solana-account-decoder = {path = "$solana_dir/account-decoder" }
solana-banks-client = { path = "$solana_dir/banks-client"}
solana-banks-server = { path = "$solana_dir/banks-server"}
solana-bpf-loader-program = { path = "$solana_dir/programs/bpf_loader" }
solana-clap-utils = {path = "$solana_dir/clap-utils" }
solana-cli-config = {path = "$solana_dir/cli-config" }
solana-cli-output = {path = "$solana_dir/cli-output" }
solana-client = { path = "$solana_dir/client"}
solana-core = { path = "$solana_dir/core"}
solana-logger = {path = "$solana_dir/logger" }
solana-notifier = { path = "$solana_dir/notifier" }
solana-remote-wallet = {path = "$solana_dir/remote-wallet" }
solana-program = { path = "$solana_dir/sdk/program" }
solana-program-test = { path = "$solana_dir/program-test" }
solana-runtime = { path = "$solana_dir/runtime" }
solana-sdk = { path = "$solana_dir/sdk" }
solana-stake-program = { path = "$solana_dir/programs/stake" }
solana-transaction-status = { path = "$solana_dir/transaction-status" }
solana-vote-program = { path = "$solana_dir/programs/vote" }
PATCH
  fi
done

./update-solana-dependencies.sh "$solana_ver"
