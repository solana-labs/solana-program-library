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
source ./ci/solana-version.sh
old_solana_ver=${solana_version#v}

sed -i'' -e "s#solana_version=v.*#solana_version=v${solana_ver}#" ./ci/solana-version.sh
sed -i'' -e "s#solana_version = \".*\"#solana_version = \"${solana_ver}\"#" ./Anchor.toml

declare tomls=()
while IFS='' read -r line; do tomls+=("$line"); done < <(find . -name Cargo.toml)

crates=(
  solana-account-decoder
  solana-banks-client
  solana-banks-interface
  solana-banks-server
  solana-bloom
  solana-bucket-map
  solana-clap-utils
  solana-clap-v3-utils
  solana-cli-config
  solana-cli-output
  solana-client
  solana-connection-cache
  solana-core
  solana-entry
  solana-faucet
  solana-frozen-abi
  solana-frozen-abi-macro
  solana-geyser-plugin-interface
  solana-geyser-plugin-manager
  solana-gossip
  solana-ledger
  solana-logger
  solana-measure
  solana-merkle-tree
  solana-metrics
  solana-net-utils
  solana-perf
  solana-poh
  solana-program-runtime
  solana-program-test
  solana-address-lookup-table-program
  solana-bpf-loader-program
  solana-compute-budget-program
  solana-config-program
  solana-stake-program
  solana-vote-program
  solana-zk-token-proof-program
  solana-pubsub-client
  solana-quic-client
  solana-rayon-threadlimit
  solana-remote-wallet
  solana-rpc
  solana-rpc-client
  solana-rpc-client-api
  solana-rpc-client-nonce-utils
  solana-runtime
  solana-sdk
  solana-sdk-macro
  solana-program
  solana-send-transaction-service
  solana-storage-bigtable
  solana-storage-proto
  solana-streamer
  solana-test-validator
  solana-thin-client
  solana-tpu-client
  solana-transaction-status
  solana-udp-client
  solana-version
  solana-zk-token-sdk
)

set -x
for crate in "${crates[@]}"; do
  sed -E -i'' -e "s:(${crate} = \")([=<>]*)${old_solana_ver}([^\"]*)\".*:\1\2${solana_ver}\3\":" "${tomls[@]}"
  sed -E -i'' -e "s:(${crate} = \{ version = \")([=<>]*)${old_solana_ver}([^\"]*)(\".*):\1\2${solana_ver}\3\4:" "${tomls[@]}"
done
