#!/usr/bin/env bash

# Script to setup a local solana-test-validator with the stake pool program

cd "$(dirname "$0")"
max_validators=$1
validator_list=$2

keys_dir=keys
mkdir -p $keys_dir
if test -f $validator_list
then
  rm $validator_list
fi

create_keypair () {
  if test ! -f $1
  then
    solana-keygen new --no-passphrase -s -o $1
  fi
}

build_program () {
  cargo build-bpf --manifest-path ../../program/Cargo.toml
}

setup_validator() {
  solana-test-validator --bpf-program SPoo1xuN9wGpxNjGnPNbRPtpQ7mHgKM8d9BeFC549Jy ../../../target/deploy/spl_stake_pool.so --quiet --reset --slots-per-epoch 32 &
  pid=$!
  solana config set --url http://127.0.0.1:8899
  solana config set --commitment confirmed
  echo "waiting for solana-test-validator, pid: $pid"
  sleep 5
}

create_vote_accounts () {
  max_validators=$1
  validator_list=$2
  for number in $(seq 1 $max_validators)
  do
    create_keypair $keys_dir/identity_$number.json
    create_keypair $keys_dir/vote_$number.json
    solana create-vote-account $keys_dir/vote_$number.json $keys_dir/identity_$number.json --commission 1
    vote_pubkey=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    echo $vote_pubkey >> $validator_list
  done
}

echo "Building on-chain program"
build_program

echo "Setting up local validator"
setup_validator

echo "Creating vote accounts"
create_vote_accounts $max_validators $validator_list
