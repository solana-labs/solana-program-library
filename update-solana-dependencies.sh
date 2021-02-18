#!/usr/bin/env bash
#
# Updates the solana version in all the SPL crates
#

solana_ver=$1
tokio_ver=$2
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
  solana-program
  solana-program-test
  solana-remote-wallet
  solana-runtime
  solana-sdk
)

set -x
for crate in "${crates[@]}"; do
  sed -i -e "s#\(${crate} = \"\).*\(\"\)#\1$solana_ver\2#g" "${tomls[@]}"
done
if [[ -n $tokio_ver ]]; then
  sed -i -e "s#\(tokio.*version *= *\"\)[^\"]*\(\".*$\)#\1$tokio_ver\2#g" "${tomls[@]}"
fi
