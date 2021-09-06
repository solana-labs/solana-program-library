#!/usr/bin/env bash

# Script to setup a stake pool, add new validators from a list

cd "$(dirname "$0")"
max_validators=$1
validator_list=$2

keys_dir=keys
spl_stake_pool=../../../target/debug/spl-stake-pool

mkdir -p $keys_dir

build_cli () {
  cargo build --manifest-path ../Cargo.toml
}

create_keypair () {
  if test ! -f $1
  then
    solana-keygen new --no-passphrase -s -o $1
  fi
}

setup_pool () {
  max_validators=$1
  stake_pool_keyfile=$2
  mint_keyfile=$3
  mkdir -p $keys_dir
  create_keypair $stake_pool_keyfile
  create_keypair $mint_keyfile

  $spl_stake_pool create-pool --fee-numerator 3 --fee-denominator 100 \
    --withdrawal-fee-numerator 5 --withdrawal-fee-denominator 1000 \
    --max-validators $max_validators \
    --pool-keypair $stake_pool_keyfile \
    --mint-keypair $mint_keyfile
}

create_validator_stakes() {
  pool=$1
  validator_list=$2
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool create-validator-stake $pool $validator
  done
}

add_validator_stakes () {
  pool=$1
  validator_list=$2
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool add-validator $pool $validator
  done
}

stake_pool_keyfile=$keys_dir/stake-pool.json
mint_keyfile=$keys_dir/mint.json

echo "Building CLI"
build_cli
echo "Creating pool"
setup_pool $max_validators $stake_pool_keyfile $mint_keyfile

stake_pool_pubkey=$(solana-keygen pubkey $stake_pool_keyfile)

echo "Creating validator stake accounts"
create_validator_stakes $stake_pool_pubkey $validator_list
echo "Adding validator stake accounts to the pool"
add_validator_stakes $stake_pool_pubkey $validator_list
