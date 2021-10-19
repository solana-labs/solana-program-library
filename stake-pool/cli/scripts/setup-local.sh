#!/usr/bin/env bash

# Script to setup a local solana-test-validator with the stake pool program
# given a maximum number of validators and a file path to store the list of
# test validator vote accounts.

cd "$(dirname "$0")"
max_validators=$1
validator_list=$2

create_keypair () {
  if test ! -f $1
  then
    solana-keygen new --no-passphrase -s -o $1
  fi
}

build_stake_pool_program () {
  cargo build-bpf --manifest-path ../../program/Cargo.toml
}

setup_test_validator() {
  solana-test-validator --bpf-program SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy ../../../target/deploy/spl_stake_pool.so --quiet --reset --slots-per-epoch 32 &
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
    create_keypair $keys_dir/withdrawer_$number.json
    solana create-vote-account $keys_dir/vote_$number.json $keys_dir/identity_$number.json $keys_dir/withdrawer_$number.json --commission 1
    vote_pubkey=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    echo $vote_pubkey >> $validator_list
  done
}


echo "Setup keys directory and clear old validator list file if found"
keys_dir=keys
mkdir -p $keys_dir
if test -f $validator_list
then
  rm $validator_list
fi

echo "Building on-chain stake pool program"
build_stake_pool_program

echo "Setting up local test validator"
setup_test_validator

echo "Creating vote accounts, these accounts be added to the stake pool"
create_vote_accounts $max_validators $validator_list
