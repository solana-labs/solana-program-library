#!/usr/bin/env bash

# Script to add a certain amount of SOL into a stake pool, given the stake pool
# keyfile and a path to a file containing a list of validator vote accounts

cd "$(dirname "$0")" || exit
stake_pool_keyfile=$1
validator_list=$2
sol_amount=$3

spl_stake_pool=spl-stake-pool
# Uncomment to use a locally build CLI
#spl_stake_pool=../../../target/debug/spl-stake-pool

increase_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  sol_amount=$3
  while read -r validator
  do
    $spl_stake_pool increase-validator-stake "$stake_pool_pubkey" "$validator" "$sol_amount"
  done < "$validator_list"
}

stake_pool_pubkey=$(solana-keygen pubkey "$stake_pool_keyfile")
echo "Increasing amount delegated to each validator in stake pool"
increase_stakes "$stake_pool_pubkey" "$validator_list" "$sol_amount"
