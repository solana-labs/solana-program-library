#!/bin/bash
echo "Running deploy script..."
SOLANA_CONFIG=$1;
PROGRAM_ID=$2;
# Get OWNER from keypair_path key of the solana config file
OWNER=`grep 'keypair_path:' $SOLANA_CONFIG | awk '{print $2}'`
MARKET_OWNER=`solana --config $SOLANA_CONFIG address`

echo "Using Solana config filepath: $SOLANA_CONFIG"
echo "Program ID: $PROGRAM_ID"
echo "OWNER: $OWNER"

solana config set --url https://api.devnet.solana.com

solana airdrop 10 $MARKET_OWNER
SOURCE=`target/debug/spl-token --config $SOLANA_CONFIG wrap 10 2>&1 | awk '{print $NF}'`

solana program --config $SOLANA_CONFIG deploy \
  --program-id $PROGRAM_ID \
  target/deploy/spl_token_lending.so

echo "Creating Lending Market"
CREATE_MARKET_OUTPUT=`target/debug/spl-token-lending create-market \
  --fee-payer    $OWNER \
  --market-owner $MARKET_OWNER \
  --verbose`

MARKET_ADDR=`echo $CREATE_MARKET_OUTPUT | head -n1 | awk '{print $4}'`

target/debug/spl-token-lending add-reserve \
  --fee-payer         $OWNER \
  --market-owner      $OWNER \
  --source-owner      $OWNER \
  --market            $MARKET_ADDR \
  --source            $SOURCE \
  --amount            5  \
  --pyth-product      3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E \
  --pyth-price        J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix \
  --switchboard-feed  AdtRGGhmqvom3Jemp5YNrxd9q9unX36BZk1pujkkXijL \
  --verbose 

# USDC Reserve
echo "Creating USDC Reserve"
USDC_TOKEN_MINT=`target/debug/spl-token --config $SOLANA_CONFIG create-token --decimals 6 |  awk '{print $3}'`
USDC_TOKEN_ACCOUNT=`target/debug/spl-token --config $SOLANA_CONFIG create-account $USDC_TOKEN_MINT | awk '{print $3}'`
target/debug/spl-token --config $SOLANA_CONFIG mint $USDC_TOKEN_MINT 30000000

target/debug/spl-token-lending add-reserve \
  --fee-payer         $OWNER \
  --market-owner      $OWNER \
  --source-owner      $OWNER \
  --market            $MARKET_ADDR \
  --source            $USDC_TOKEN_ACCOUNT \
  --amount            1000000  \
  --pyth-product      6NpdXrQEpmDZ3jZKmM2rhdmkd3H6QAk23j2x8bkXcHKA \
  --pyth-price        5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7 \
  --switchboard-feed  CZx29wKMUxaJDq6aLVQTdViPL754tTR64NAgQBUGxxHb \
  --verbose


# SRM Reserve
echo "Creating SRM Reserve"
SRM_TOKEN_MINT=`target/debug/spl-token --config $SOLANA_CONFIG create-token --decimals 6 |  awk '{print $3}'`
SRM_TOKEN_ACCOUNT=`target/debug/spl-token --config $SOLANA_CONFIG create-account $SRM_TOKEN_MINT | awk '{print $3}'`
target/debug/spl-token --config $SOLANA_CONFIG mint $SRM_TOKEN_MINT 8000000 

target/debug/spl-token-lending add-reserve \
  --fee-payer         $OWNER \
  --market-owner      $OWNER \
  --source-owner      $OWNER \
  --market            $MARKET_ADDR \
  --source            $SRM_TOKEN_ACCOUNT \
  --amount            5000000  \
  --pyth-product      6MEwdxe4g1NeAF9u6KDG14anJpFsVEa2cvr5H6iriFZ8 \
  --pyth-price        992moaMQKs32GKZ9dxi8keyM2bUmbrwBZpK4p2K6X5Vs \
  --switchboard-feed  BAoygKcKN7wk8yKzLD6sxzUQUqLvhBV1rjMA4UJqfZuH \
  --verbose

target/debug/spl-token --config $SOLANA_CONFIG unwrap
