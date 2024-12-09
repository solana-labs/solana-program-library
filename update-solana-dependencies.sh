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
  solana-account-decoder-client-types
  solana-banks-client
  solana-banks-interface
  solana-banks-server
  solana-bloom
  solana-bucket-map
  solana-builtins-default-costs
  solana-clap-utils
  solana-clap-v3-utils
  solana-cli-config
  solana-cli-output
  solana-client
  solana-compute-budget
  solana-connection-cache
  solana-core
  solana-entry
  solana-faucet
  solana-fee
  solana-frozen-abi
  solana-frozen-abi-macro
  agave-geyser-plugin-interface
  solana-geyser-plugin-manager
  solana-gossip
  solana-lattice-hash
  solana-ledger
  solana-log-collector
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
  solana-system-program
  solana-vote-program
  solana-zk-elgamal-proof-program
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
  solana-runtime-transaction
  solana-sdk
  solana-sdk-macro
  solana-program
  solana-send-transaction-service
  solana-storage-bigtable
  solana-storage-proto
  solana-streamer
  solana-svm-rent-collector
  solana-svm-transaction
  solana-test-validator
  solana-thin-client
  solana-tpu-client
  solana-transaction-status
  solana-transaction-status-client-types
  solana-udp-client
  solana-version
  solana-zk-token-sdk
  solana-zk-sdk
  solana-bn254
  solana-curve25519
  solana-secp256k1-recover
  solana-account
  solana-account-info
  solana-atomic-u64
  solana-bincode
  solana-borsh
  solana-clock
  solana-cpi
  solana-decode-error
  solana-define-syscall
  solana-derivation-path
  solana-epoch-schedule
  solana-feature-set
  solana-fee-calculator
  solana-hash
  solana-inflation
  solana-instruction
  solana-last-restart-slot
  solana-msg
  solana-native-token
  solana-packet
  solana-precompile-error
  solana-program-entrypoint
  solana-program-error
  solana-program-memory
  solana-program-option
  solana-program-pack
  solana-pubkey
  solana-rent
  solana-sanitize
  solana-serde-varint
  solana-serialize-utils
  solana-sha256-hasher
  solana-short-vec
  solana-signature
  solana-slot-hashes
  solana-stable-layout
  solana-timings
  solana-transaction-error
)

set -x
for crate in "${crates[@]}"; do
  sed -E -i'' -e "s:(${crate} = \")([=<>]*)${old_solana_ver}([^\"]*)\".*:\1\2${solana_ver}\3\":" "${tomls[@]}"
  sed -E -i'' -e "s:(${crate} = \{ version = \")([=<>]*)${old_solana_ver}([^\"]*)(\".*):\1\2${solana_ver}\3\4:" "${tomls[@]}"
done
