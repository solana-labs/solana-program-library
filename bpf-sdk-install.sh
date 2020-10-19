#!/usr/bin/env bash
set -e

channel=${1:-v1.3.17}
installDir="$(dirname "$0")"/bin
cacheDir=~/.cache/solana-bpf-sdk/"$channel"

echo "Installing $channel BPF SDK into $installDir"

set -x

if [[ ! -r "$cacheDir"/bpf-sdk.tar.bz2 ]]; then
  mkdir -p "$cacheDir"
  curl -L --retry 5 --retry-delay 2 -o "$cacheDir"/bpf-sdk.tar.bz2 \
    https://solana-sdk.s3.amazonaws.com/"$channel"/bpf-sdk.tar.bz2
fi

rm -rf "$installDir"
mkdir -p "$installDir"
(
  cd "$installDir"
  tar jxf "$cacheDir"/bpf-sdk.tar.bz2
)
cat "$installDir"/bpf-sdk/version.txt
