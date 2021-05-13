#!/usr/bin/env bash

user1_seed_words="only region spot moral grab cigar isolate fragile find woman slam pitch clap bind release hospital choice project state million there oyster wine try"
user1="FWRE4usb2MsRxnes6myJJ7tFZEw3ZDMRyf2YbPjGTp28"
user2_seed_words="rapid prepare axis cross average carry mention unable door shallow voyage way recall over fossil renew minimum van craft choose crater nothing visit train"
user2="DMncFaHuRRePtJFvJ2WQ7HUyTBFv8Y9sbSF8g8Ffa8Xa"

echo "setting up mint"
solana config set -u localhost
solana-keygen new --no-passphrase -s -o mint.json
mint=$(solana-keygen pubkey mint.json)
cargo run -- create-token mint.json --enable-freeze

echo "setting up user 1 account"
solana-keygen new --no-passphrase -s -o user1_token.json
user1_token=$(solana-keygen pubkey user1_token.json)
cargo run -- create-account $mint user1_token.json --owner $user1
cargo run -- mint $mint 30 $user1_token

echo "setting up user 2 account"
solana-keygen new --no-passphrase -s -o user2_token.json
user2_token=$(solana-keygen pubkey user2_token.json)
cargo run -- create-account $mint user2_token.json --owner $user2
cargo run -- mint $mint 30 $user2_token

echo "Go to sollet web, transfer 10 tokens from $user1_token account to $user2_token"
read DONE

echo "Performing clawback"
cargo run -- clawback $user2_token 10 $user1_token

echo "Note the signature, then go to explorer.solana.com and show the transaction"
