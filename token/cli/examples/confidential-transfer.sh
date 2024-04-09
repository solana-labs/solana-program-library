#!/usr/bin/env bash

# Set whichever network you would like to test with
# solana config set -ul

program_id="TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"

echo "Setup keypairs"
solana-keygen new -o confidential-mint.json --no-bip39-passphrase
solana-keygen new -o confidential-source.json --no-bip39-passphrase
solana-keygen new -o confidential-destination.json --no-bip39-passphrase
mint_pubkey=$(solana-keygen pubkey "confidential-mint.json")
source_pubkey=$(solana-keygen pubkey "confidential-source.json")
destination_pubkey=$(solana-keygen pubkey "confidential-destination.json")

set -ex
echo "Initializing mint"
spl-token --program-id "$program_id" create-token confidential-mint.json --enable-confidential-transfers auto
echo "Displaying"
spl-token display "$mint_pubkey"
read  -n 1 -p "..."

echo "Setting up transfer accounts"
spl-token create-account "$mint_pubkey" confidential-source.json
spl-token configure-confidential-transfer-account --address "$source_pubkey"
spl-token create-account "$mint_pubkey" confidential-destination.json
spl-token configure-confidential-transfer-account --address "$destination_pubkey"
spl-token mint "$mint_pubkey" 100 confidential-source.json

echo "Displaying"
spl-token display "$source_pubkey"
read  -n 1 -p "..."

echo "Depositing into confidential"
spl-token deposit-confidential-tokens "$mint_pubkey" 100 --address "$source_pubkey"
echo "Displaying"
spl-token display "$source_pubkey"
read  -n 1 -p "..."

echo "Applying pending balances"
spl-token apply-pending-balance --address "$source_pubkey"
echo "Displaying"
spl-token display "$source_pubkey"
read  -n 1 -p "..."

echo "Transferring 10"
spl-token transfer "$mint_pubkey" 10 "$destination_pubkey" --from "$source_pubkey" --confidential
echo "Displaying source"
spl-token display "$source_pubkey"
echo "Displaying destination"
spl-token display "$destination_pubkey"
read  -n 1 -p "..."

echo "Applying balance on destination"
spl-token apply-pending-balance --address "$destination_pubkey"
echo "Displaying destination"
spl-token display "$destination_pubkey"
read  -n 1 -p "..."

echo "Transferring 0"
spl-token transfer "$mint_pubkey" 0 "$destination_pubkey" --from "$source_pubkey" --confidential
echo "Displaying destination"
spl-token display "$destination_pubkey"
read  -n 1 -p "..."

echo "Transferring 0 again"
spl-token transfer "$mint_pubkey" 0 "$destination_pubkey" --from "$source_pubkey" --confidential
echo "Displaying destination"
spl-token display "$destination_pubkey"
read  -n 1 -p "..."

echo "Withdrawing 10 from destination"
spl-token apply-pending-balance --address "$destination_pubkey"
spl-token withdraw-confidential-tokens "$mint_pubkey" 10 --address "$destination_pubkey"
echo "Displaying destination"
spl-token display "$destination_pubkey"
read  -n 1 -p "..."
