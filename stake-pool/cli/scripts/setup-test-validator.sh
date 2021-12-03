#!/usr/bin/env bash

# Script to setup a local solana-test-validator with the stake pool program
# given a maximum number of validators and a file path to store the list of
# test validator vote accounts.

cd "$(dirname "$0")" || exit
max_validators=$1
validator_file=$2

create_keypair () {
  if test ! -f "$1"
  then
    solana-keygen new --no-passphrase -s -o "$1"
  fi
}

setup_test_validator() {
  solana-test-validator -c SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy -c EmiU8AQkB2sswTxVB6aCmsAJftoowZGGDXuytm6X65R3 --url devnet --slots-per-epoch 32 --quiet --reset &
  pid=$!
  solana config set --url http://127.0.0.1:8899
  solana config set --commitment confirmed
  echo "waiting for solana-test-validator, pid: $pid"
  sleep 5
}

create_vote_accounts () {
  max_validators=$1
  validator_file=$2
  for number in $(seq 1 "$max_validators")
  do
    create_keypair "$keys_dir/identity_$number.json"
    create_keypair "$keys_dir/vote_$number.json"
    create_keypair "$keys_dir/withdrawer_$number.json"
    solana create-vote-account "$keys_dir/vote_$number.json" "$keys_dir/identity_$number.json" "$keys_dir/withdrawer_$number.json" --commission 1
    vote_pubkey=$(solana-keygen pubkey "$keys_dir/vote_$number.json")
    echo "$vote_pubkey" >> "$validator_file"
  done
}


echo "Setup keys directory and clear old validator list file if found"
keys_dir=keys
mkdir -p $keys_dir
if test -f "$validator_file"
then
  rm "$validator_file"
fi

echo "Setting up local test validator"
setup_test_validator

echo "Creating vote accounts, these accounts be added to the stake pool"
create_vote_accounts "$max_validators" "$validator_file"

echo "Done adding $max_validators validator vote accounts, their pubkeys can be found in $validator_file"
