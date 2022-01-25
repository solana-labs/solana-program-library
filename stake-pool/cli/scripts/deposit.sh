#!/usr/bin/env bash

# Script to deposit stakes and SOL into a stake pool, given the stake pool keyfile
# and a path to a file containing a list of validator vote accounts

cd "$(dirname "$0")" || exit
stake_pool_keyfile=$1
validator_list=$2
sol_amount=$3

create_keypair () {
  if test ! -f "$1"
  then
    solana-keygen new --no-passphrase -s -o "$1"
  fi
}

create_user_stakes () {
  validator_list=$1
  sol_amount=$2
  authority=$3
  while read -r validator
  do
    create_keypair "$keys_dir/stake_$validator".json
    solana create-stake-account "$keys_dir/stake_$validator.json" "$sol_amount" --withdraw-authority "$authority" --stake-authority "$authority"
  done < "$validator_list"
}

delegate_user_stakes () {
  validator_list=$1
  authority=$2
  while read -r validator
  do
    solana delegate-stake --force "$keys_dir/stake_$validator.json" "$validator" --stake-authority "$authority"
  done < "$validator_list"
}

deposit_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  authority=$3
  while read -r validator
  do
    stake=$(solana-keygen pubkey "$keys_dir/stake_$validator.json")
    $spl_stake_pool deposit-stake "$stake_pool_pubkey" "$stake" --withdraw-authority "$authority"
  done < "$validator_list"
}

keys_dir=keys
stake_pool_pubkey=$(solana-keygen pubkey "$stake_pool_keyfile")

spl_stake_pool=spl-stake-pool
# Uncomment to use a locally build CLI
#spl_stake_pool=../../../target/debug/spl-stake-pool

echo "Setting up keys directory $keys_dir"
mkdir -p $keys_dir
authority=$keys_dir/authority.json
echo "Setting up authority for deposited stake accounts at $authority"
create_keypair $authority

echo "Creating user stake accounts to deposit into the pool"
create_user_stakes "$validator_list" "$sol_amount" $authority
echo "Delegating user stakes so that deposit will work"
delegate_user_stakes "$validator_list" $authority

echo "Waiting for stakes to activate, this may take awhile depending on the network!"
echo "If you are running on localnet with 32 slots per epoch, wait 12 seconds..."
sleep 12
echo "Depositing stakes into stake pool"
deposit_stakes "$stake_pool_pubkey" "$validator_list" $authority
echo "Depositing SOL into stake pool"
$spl_stake_pool deposit-sol "$stake_pool_pubkey" "$sol_amount"
