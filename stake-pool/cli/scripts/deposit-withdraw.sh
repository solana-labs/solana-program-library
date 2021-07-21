#!/usr/bin/env bash

# Script to deposit and withdraw stakes from a pool, given stake pool public key
# and a list of validators

cd "$(dirname "$0")"
stake_pool_keyfile=$1
validator_list=$2

stake_pool_pubkey=$(solana-keygen pubkey $stake_pool_keyfile)

sol_amount=2
half_sol_amount=1
keys_dir=keys
spl_stake_pool=../../../target/debug/spl-stake-pool

mkdir -p $keys_dir

create_keypair () {
  if test ! -f $1
  then
    solana-keygen new --no-passphrase -s -o $1
  fi
}

create_user_stakes () {
  validator_list=$1
  sol_amount=$2
  for validator in $(cat $validator_list)
  do
    create_keypair $keys_dir/stake_$validator.json
    solana create-stake-account $keys_dir/stake_$validator.json $sol_amount
  done
}

delegate_user_stakes () {
  validator_list=$1
  for validator in $(cat $validator_list)
  do
    solana delegate-stake --force $keys_dir/stake_$validator.json $validator
  done
}

deposit_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  for validator in $(cat $validator_list)
  do
    stake=$(solana-keygen pubkey $keys_dir/stake_$validator.json)
    $spl_stake_pool deposit-stake $stake_pool_pubkey $stake
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

echo "Creating user stake accounts"
create_user_stakes $validator_list $sol_amount
echo "Delegating user stakes"
delegate_user_stakes $validator_list
echo "Waiting for stakes to activate, this may take awhile depending on the network!"
echo "If you are running on localnet with 32 slots per epoch, wait 24 seconds..."
sleep 24
echo "Depositing stakes into stake pool"
deposit_stakes $stake_pool_pubkey $validator_list
echo "Withdrawing stakes from stake pool"
withdraw_stakes $stake_pool_pubkey $validator_list $half_sol_amount
