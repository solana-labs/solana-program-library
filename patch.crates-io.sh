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

crates_map=()
crates_map+=("solana-account-decoder account-decoder")
crates_map+=("solana-banks-client banks-client")
crates_map+=("solana-banks-server banks-server")
crates_map+=("solana-bpf-loader-program programs/bpf_loader")
crates_map+=("solana-clap-utils clap-utils")
crates_map+=("solana-cli-config cli-config")
crates_map+=("solana-cli-output cli-output")
crates_map+=("solana-client client")
crates_map+=("solana-core core")
crates_map+=("solana-logger logger")
crates_map+=("solana-notifier notifier")
crates_map+=("solana-remote-wallet remote-wallet")
crates_map+=("solana-program sdk/program")
crates_map+=("solana-program-test program-test")
crates_map+=("solana-runtime runtime")
crates_map+=("solana-sdk sdk")
crates_map+=("solana-stake-program programs/stake")
crates_map+=("solana-test-validator test-validator")
crates_map+=("solana-transaction-status transaction-status")
crates_map+=("solana-version version")
crates_map+=("solana-vote-program programs/vote")
crates_map+=("solana-zk-token-sdk zk-token-sdk")

patch_crates=()
for map_entry in "${crates_map[@]}"; do
  read -r crate_name crate_path <<<"$map_entry"
  full_path="$solana_dir/$crate_path"
  if [[ -r "$full_path/Cargo.toml" ]]; then
    patch_crates+=("$crate_name = { path = \"$full_path\" }")
  fi
done

echo "Patching in $solana_ver from $solana_dir"
echo
for crate in "${workspace_crates[@]}"; do
  if grep -q '\[patch.crates-io\]' "$crate"; then
    echo "$crate is already patched"
  else
    cat >> "$crate" <<PATCH
[patch.crates-io]
$(printf "%s\n" "${patch_crates[@]}")
PATCH
  fi
done

./update-solana-dependencies.sh "$solana_ver"
