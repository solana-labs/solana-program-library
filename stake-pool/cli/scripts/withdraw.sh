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

create_stake_account () {
  authority=$1
  while read -r validator
  do
    solana-keygen new --no-passphrase -o "$keys_dir/stake_account_$validator.json"
    solana create-stake-account "$keys_dir/stake_account_$validator.json" 2 
    solana delegate-stake --force "$keys_dir/stake_account_$validator.json"  "$validator" 
  done < "$validator_list"
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

withdraw_stakes_to_stake_receiver () {
  stake_pool_pubkey=$1
  validator_list=$2
  pool_amount=$3
  while read -r validator
  do
    stake_receiver=$(solana-keygen pubkey "$keys_dir/stake_account_$validator.json")
    $spl_stake_pool withdraw-stake "$stake_pool_pubkey" "$pool_amount" --vote-account "$validator" --stake-receiver "$stake_receiver"
  done < "$validator_list"
}

spl_stake_pool=spl-stake-pool
# Uncomment to use a locally build CLI
# spl_stake_pool=../../../target/debug/spl-stake-pool

stake_pool_pubkey=$(solana-keygen pubkey "$stake_pool_keyfile")
keys_dir=keys

echo "Setting up keys directory $keys_dir"
mkdir -p $keys_dir
authority=$keys_dir/authority.json

create_stake_account $authority
echo "Waiting for stakes to activate, this may take awhile depending on the network!"
echo "If you are running on localnet with 32 slots per epoch, wait 12 seconds..."
sleep 12

echo "Setting up authority for withdrawn stake accounts at $authority"
create_keypair $authority

echo "Withdrawing stakes from stake pool"
withdraw_stakes "$stake_pool_pubkey" "$validator_list" "$withdraw_sol_amount"

echo "Withdrawing stakes from stake pool to receive it in stake receiver account"
withdraw_stakes_to_stake_receiver "$stake_pool_pubkey" "$validator_list" "$withdraw_sol_amount"

echo "Withdrawing SOL from stake pool to authority"
$spl_stake_pool withdraw-sol "$stake_pool_pubkey" $authority "$withdraw_sol_amount"
