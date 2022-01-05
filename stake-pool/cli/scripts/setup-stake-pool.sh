#!/usr/bin/env bash

# Script to setup a stake pool from scratch.  Please modify the parameters to
# create a stake pool to your liking!

cd "$(dirname "$0")" || exit
command_args=()

###################################################
### MODIFY PARAMETERS BELOW THIS LINE FOR YOUR POOL
###################################################

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

command_args+=( --max-validators 2950 ) # Maximum number of validators in the stake pool, 2950 is the current maximum possible

# (Optional) Deposit authority, required to sign all deposits into the pool.
# Setting this variable makes the pool "private" or "restricted".
# Uncomment and set to a valid keypair if you want the pool to be restricted.
#command_args+=( --deposit-authority keys/authority.json )

###################################################
### MODIFY PARAMETERS ABOVE THIS LINE FOR YOUR POOL
###################################################

keys_dir=keys
spl_stake_pool=spl-stake-pool
# Uncomment to use a local build
#spl_stake_pool=../../../target/debug/spl-stake-pool

mkdir -p $keys_dir

create_keypair () {
  if test ! -f "$1"
  then
    solana-keygen new --no-passphrase -s -o "$1"
  fi
}

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
  create-pool \
  "${command_args[@]}" \
  --pool-keypair "$stake_pool_keyfile" \
  --validator-list-keypair "$validator_list_keyfile" \
  --mint-keypair "$mint_keyfile" \
  --reserve-keypair "$reserve_keyfile"
