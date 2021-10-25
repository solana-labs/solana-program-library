#!/usr/bin/env bash

# Script to setup a stake pool, add new validators from a list

cd "$(dirname "$0")"
global_args=()
command_args=()

###################################################
### MODIFY PARAMETERS BELOW THIS LINE FOR YOUR POOL
###################################################

global_args+=( --manager keys/new_manager.json ) # Keypair of the manager of the stake pool
global_args+=( --staker keys/new_staker.json ) # Keypair of the staker of the stake pool

# Epoch fee, assessed as a percentage of rewards earned by the pool every epoch,
# represented as `numerator / denominator`
command_args+=( --epoch-fee-numerator 0 )
command_args+=( --epoch-fee-denominator 0 )

# Withdrawal fee for SOL and stake accounts, represented as `numerator / denominator`
command_args+=( --withdrawal-fee-numerator 0 )
command_args+=( --withdrawal-fee-denominator 0 )

# Deposit fee for SOL and stake accounts, represented as `numerator / denominator`
command_args+=( --deposit-fee-numerator 0 )
command_args+=( --deposit-fee-denominator 0 )

command_args+=( --referral-fee 0 ) # Percentage of deposit fee that goes towards the referrer (a number between 0 and 100, inclusive)

command_args+=( --max-validators 3950 ) # Maximum number of validators in the stake pool, 3950 is the current maximum possible

validator_list=validator_list.txt # File containing validator vote account addresses, each will be added to the stake pool after creation

# (Optional) Deposit authority, required to sign all deposits into the pool.
# Setting this variable makes the pool "private" or "restricted".
# Comment it out if you want the pool to be open to all depositors.
command_args+=( --deposit-authority keys/authority.json )

###################################################
### MODIFY PARAMETERS ABOVE THIS LINE FOR YOUR POOL
###################################################

keys_dir=keys
spl_stake_pool=../../../target/debug/spl-stake-pool

mkdir -p $keys_dir

build_stake_pool_cli () {
  cargo build --manifest-path ../Cargo.toml
}

create_keypair () {
  if test ! -f $1
  then
    solana-keygen new --no-passphrase -s -o $1
  fi
}

add_validator_stakes () {
  pool=$1
  validator_list=$2
  for validator in $(cat $validator_list)
  do
    $spl_stake_pool "${global_args[@]}" add-validator $pool $validator
  done
}

echo "Building stake pool CLI"
build_stake_pool_cli

echo "Creating pool"
stake_pool_keyfile=$keys_dir/stake-pool.json
validator_list_keyfile=$keys_dir/validator-list.json
mint_keyfile=$keys_dir/mint.json
reserve_keyfile=$keys_dir/reserve.json
create_keypair $stake_pool_keyfile
create_keypair $validator_list_keyfile
create_keypair $mint_keyfile
create_keypair $reserve_keyfile

set -ex
$spl_stake_pool \
  "${global_args[@]}" \
  create-pool \
  "${command_args[@]}" \
  --pool-keypair "$stake_pool_keyfile" \
  --validator-list-keypair "$validator_list_keyfile" \
  --mint-keypair "$mint_keyfile" \
  --reserve-keypair "$reserve_keyfile"

stake_pool_pubkey=$(solana-keygen pubkey $stake_pool_keyfile)

echo "Adding validator stake accounts to the pool"
add_validator_stakes $stake_pool_pubkey $validator_list
