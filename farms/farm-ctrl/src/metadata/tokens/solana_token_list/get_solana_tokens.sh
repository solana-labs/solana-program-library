#!/usr/bin/env bash

# Solana tokens loader

url="https://raw.githubusercontent.com/solana-labs/token-list/main/src/tokens/solana.tokenlist.json"

if hash wget 2>/dev/null; then
  wget_or_curl="wget -O tokens.json $url"
elif hash curl 2>/dev/null; then
  wget_or_curl="curl -o tokens.json -L $url"
else
  echo "Error: Neither curl nor wget were found" >&2
  return 1
fi

exec $wget_or_curl
