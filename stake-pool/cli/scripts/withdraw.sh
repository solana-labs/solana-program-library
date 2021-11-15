#!/usr/bin/env bash

# Script to withdraw stakes and SOL from a stake pool, given the stake pool public key
# and a path to a file containing a list of validator vote accounts

cd "$(dirname "$0")" || exit
stake_pool_keyfile=$1
validator_list=$2
withdraw_sol_amount=$3

create_keypair () {
  if test ! -f "$1"
  then
    solana-keygen new --no-passphrase -s -o "$1"
  fi
}

withdraw_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  pool_amount=$3
  while read -r validator
  do
    $spl_stake_pool withdraw-stake "$stake_pool_pubkey" "$pool_amount" --vote-account "$validator"
  done < "$validator_list"
}

stake_pool_pubkey=$(solana-keygen pubkey "$stake_pool_keyfile")
keys_dir=keys

spl_stake_pool=spl-stake-pool
# Uncomment to use a locally build CLI
#spl_stake_pool=../../../target/debug/spl-stake-pool

echo "Setting up keys directory $keys_dir"
mkdir -p $keys_dir
authority=$keys_dir/authority.json
echo "Setting up authority for withdrawn stake accounts at $authority"
create_keypair $authority

echo "Withdrawing stakes from stake pool"
withdraw_stakes "$stake_pool_pubkey" "$validator_list" "$withdraw_sol_amount"
echo "Withdrawing SOL from stake pool to authority"
$spl_stake_pool withdraw-sol "$stake_pool_pubkey" $authority "$withdraw_sol_amount"
