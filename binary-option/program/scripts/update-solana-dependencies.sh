#!/usr/bin/env bash
#
# Updates the solana version in all the SPL crates
#

here="$(dirname "$0")"

solana_ver=$1
if [[ -z $solana_ver ]]; then
  echo "Usage: $0 <new-solana-version>"
  exit 1
fi

if [[ $solana_ver =~ ^v ]]; then
  # Drop `v` from v1.2.3...
  solana_ver=${solana_ver:1}
fi

cd "$here"/..

echo "Updating Solana version to $solana_ver in $PWD"

if ! git diff --quiet && [[ -z $DIRTY_OK ]]; then
  echo "Error: dirty tree"
  exit 1
fi

declare tomls=()
while IFS='' read -r line; do tomls+=("$line"); done < <(find . -name Cargo.toml)

crates=(
  solana-clap-utils
  solana-cli-config
  solana-client
  solana-logger
  solana-program
  solana-program-test
  solana-remote-wallet
  solana-sdk
  solana-validator
)

set -x
for crate in "${crates[@]}"; do
  sed -i -e "s#\(${crate} = \"\).*\(\"\)#\1$solana_ver\2#g" "${tomls[@]}"
done

