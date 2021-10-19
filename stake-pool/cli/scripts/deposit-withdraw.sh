#!/usr/bin/env bash

# Script to deposit and withdraw stakes from a pool, given stake pool public key
# and a path to a file containing a list of validator vote accounts

cd "$(dirname "$0")"
stake_pool_keyfile=$1
validator_list=$2

create_keypair () {
  if test ! -f $1
  then
    solana-keygen new --no-passphrase -s -o $1
  fi
}

create_user_stakes () {
  validator_list=$1
  sol_amount=$2
  authority=$3
  for validator in $(cat $validator_list)
  do
    create_keypair $keys_dir/stake_$validator.json
    solana create-stake-account $keys_dir/stake_$validator.json $sol_amount --withdraw-authority $authority --stake-authority $authority
  done
}

delegate_user_stakes () {
  validator_list=$1
  authority=$2
  for validator in $(cat $validator_list)
  do
    solana delegate-stake --force $keys_dir/stake_$validator.json $validator --stake-authority $authority
  done
}

deposit_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  authority=$3
  for validator in $(cat $validator_list)
  do
    stake=$(solana-keygen pubkey $keys_dir/stake_$validator.json)
    $spl_stake_pool deposit-stake $stake_pool_pubkey $stake --withdraw-authority $authority
  done
}

withdraw_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  pool_amount=$3
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool withdraw-stake $stake_pool_pubkey $pool_amount --vote-account $validator
  done
}

sol_amount=2
half_sol_amount=1
keys_dir=keys
spl_stake_pool=../../../target/debug/spl-stake-pool
stake_pool_pubkey=$(solana-keygen pubkey $stake_pool_keyfile)
echo "Setting up keys directory $keys_dir"
mkdir -p $keys_dir
authority=$keys_dir/authority.json
echo "Setting up authority for deposited stake accounts at $authority"
create_keypair $authority

echo "Creating user stake accounts to deposit into the pool"
create_user_stakes $validator_list $sol_amount $authority
echo "Delegating user stakes so that deposit will work"
delegate_user_stakes $validator_list $authority
echo "Waiting for stakes to activate, this may take awhile depending on the network!"
echo "If you are running on localnet with 32 slots per epoch, wait 12 seconds..."
sleep 12
echo "Depositing stakes into stake pool"
deposit_stakes $stake_pool_pubkey $validator_list $authority
echo "Withdrawing stakes from stake pool"
withdraw_stakes $stake_pool_pubkey $validator_list $half_sol_amount
echo "Depositing SOL into stake pool to authority"
$spl_stake_pool deposit-sol $stake_pool_pubkey $sol_amount
echo "Withdrawing SOL from stake pool to authority"
$spl_stake_pool withdraw-sol $stake_pool_pubkey $authority $half_sol_amount
