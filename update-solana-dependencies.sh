#!/usr/bin/env bash
#
# Updates the solana version in all the SPL crates
#

solana_ver=$1
if [[ -z $solana_ver ]]; then
  echo "Usage: $0 <new-solana-version>"
  exit 1
fi

cd "$(dirname "$0")"

declare tomls=()
while IFS='' read -r line; do tomls+=("$line"); done < <(find . -name Cargo.toml)

crates=(
  solana-account-decoder
  solana-banks-client
  solana-banks-server
  solana-bpf-loader-program
  solana-clap-utils
  solana-cli-config
  solana-cli-output
  solana-client
  solana-core
  solana-logger
  solana-notifier
  solana-program
  solana-program-test
  solana-remote-wallet
  solana-runtime
  solana-sdk
  solana-stake-program
  solana-transaction-status
  solana-vote-program
)

set -x
for crate in "${crates[@]}"; do
  sed -i -e "s#\(${crate} = \"\)\(=\?\).*\(\"\)#\1\2$solana_ver\3#g" "${tomls[@]}"
done
