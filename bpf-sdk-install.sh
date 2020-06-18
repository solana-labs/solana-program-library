#!/usr/bin/env bash
set -e

installDir=$1
channel=v1.2.3

if [[ -n $2 ]]; then
  channel=$2
fi

echo "Installing $channel BPF SDK into $installDir"

set -x
cd "$installDir/"
curl -L  --retry 5 --retry-delay 2 -o bpf-sdk.tar.bz2 \
  http://solana-sdk.s3.amazonaws.com/"$channel"/bpf-sdk.tar.bz2
rm -rf bpf-sdk
mkdir -p bpf-sdk
tar jxf bpf-sdk.tar.bz2
rm -f bpf-sdk.tar.bz2

cat bpf-sdk/version.txt
