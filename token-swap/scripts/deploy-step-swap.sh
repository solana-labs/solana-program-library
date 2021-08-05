#!/usr/bin/env bash
set -ex

if [[ -z "${2}" ]]; then
    echo "Error: program keypair not set"
    exit
fi

if [[ -z "${3}" ]]; then
    echo "Error: deployer keypair not set"
    exit
fi

if [ ! -d "../../target/deploy" ]; then
    ./do.sh build-prod
fi

if ! hash solana 2>/dev/null; then
    ./do.sh update
fi

keypair="$HOME"/.config/solana/id.json
if [ ! -f "$keypair" ]; then
    echo Generating keypair ...
    solana-keygen new -o "$keypair" --no-passphrase --silent
fi

CLUSTER_URL=""
if [[ $1 == "localnet" ]]; then
    CLUSTER_URL="http://localhost:8899"
elif [[ $1 == "devnet" ]]; then
    CLUSTER_URL="https://api.devnet.solana.com"
elif [[ $1 == "testnet" ]]; then
    CLUSTER_URL="https://api.testnet.solana.com"
else
    echo "Unsupported network: $1"
    exit 1
fi

solana config set --url $CLUSTER_URL
sleep 1
solana airdrop 10
solana program deploy --program-id ${2} --keypair ${3} ../../target/deploy/spl_token_swap.so
