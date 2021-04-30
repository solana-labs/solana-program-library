#!/usr/bin/env bash

set -ex

keys_dir=keys
pid=

setup () {
  max_validators=$1
  solana-test-validator --bpf-program poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj ../../target/deploy/spl_stake_pool.so --quiet --reset --slots-per-epoch 32 &
  pid=$!
  sleep 5
  solana config set --url http://127.0.0.1:8899
  mkdir -p $keys_dir
  create_keypair $keys_dir/stake-pool.json
  create_keypair $keys_dir/mint.json

  cargo run -- create-pool --fee-numerator 3 --fee-denominator 100 --max-validators $max_validators --pool-keypair $keys_dir/stake-pool.json --mint-keypair $keys_dir/mint.json
}

# This is gross, but with such short epochs, we often run the risk of checking
# for an update, and then passing the epoch boundary right after that, causing
# the instruction to fail. To get around it, we just run the command twice.
repeat_command () {
  set +e
  cmd=$1
  eval $cmd
  status=$?
  if test $status -ne 0
  then
    eval $cmd
    status=$?
    if test $status -ne 0
    then
      exit 1
    fi
  fi
  set -e
}

create_keypair () {
  set +ex
  solana-keygen new --no-passphrase -s -o $1
  set -ex
}

create_votes () {
  validators=$1
  for number in $(seq 1 $validators)
  do
    create_keypair $keys_dir/identity_$number.json
    create_keypair $keys_dir/vote_$number.json
    solana create-vote-account $keys_dir/vote_$number.json $keys_dir/identity_$number.json --commission 1
  done
}

create_stakes () {
  validators=$1
  pool=$2
  sol_amount=$3
  for number in $(seq 1 $validators)
  do
    validator=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    cargo run -- create-validator-stake $pool $validator
    create_keypair $keys_dir/stake_$number.json
    solana create-stake-account $keys_dir/stake_$number.json $sol_amount
    solana delegate-stake --force $keys_dir/stake_$number.json $validator
  done
}

add_stakes () {
  validators=$1
  pool=$2
  for number in $(seq 1 $validators)
  do
    validator=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    repeat_command "cargo run -- add-validator $pool $validator"
  done
}

deposit_stakes () {
  validators=$1
  pool=$2
  for number in $(seq 1 $validators)
  do
    stake=$(solana-keygen pubkey $keys_dir/stake_$number.json)
    repeat_command "cargo run -- deposit $pool $stake"
  done
}

withdraw_stakes () {
  validators=$1
  pool=$2
  pool_amount=$3
  for number in $(seq 1 $validators)
  do
    vote=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    repeat_command "cargo run -- withdraw $pool $pool_amount --vote-account $vote"
  done
}

decrease_stakes () {
  validators=$1
  pool=$2
  sol_amount=$3
  for number in $(seq 1 $validators)
  do
    stake=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    repeat_command "cargo run -- decrease-validator-stake $pool $stake $sol_amount"
  done
}

increase_stakes () {
  validators=$1
  pool=$2
  sol_amount=$3
  for number in $(seq 1 $validators)
  do
    stake=$(solana-keygen pubkey $keys_dir/vote_$number.json)
    repeat_command "cargo run -- increase-validator-stake $pool $stake $sol_amount"
  done
}

sol_amount=10
half_sol_amount=5
max_validators=10
echo "Setting up"
setup $max_validators

pool=$(solana-keygen pubkey $keys_dir/stake-pool.json)
mint=$(solana-keygen pubkey $keys_dir/mint.json)

echo "Creating vote accounts"
create_votes $max_validators
echo "Creating validator and user stake accounts"
create_stakes $max_validators $pool $sol_amount
echo "Waiting for stakes to activate"
sleep 20
echo "Adding validator stake accounts to the pool"
add_stakes $max_validators $pool
echo "Depositing into stake pool"
deposit_stakes $max_validators $pool
echo "Decreasing stakes"
decrease_stakes $max_validators $pool $half_sol_amount
echo "Waiting for transient stakes to deactivate"
sleep 10
echo "Increasing stakes"
increase_stakes $max_validators $pool $half_sol_amount
echo "Waiting for transient stakes to activate"
sleep 20
echo "Withdrawing stakes"
withdraw_stakes $max_validators $pool $half_sol_amount
echo "Decrease all stakes to withdraw from reserve"
decrease_stakes $max_validators $pool $half_sol_amount
echo "Waiting for transient stakes to deactivate"
sleep 10
echo "Withdrawing from reserve"
repeat_command "cargo run -- withdraw $pool $half_sol_amount --use-reserve"

echo "All done, cleaning up!"
kill -9 $pid
