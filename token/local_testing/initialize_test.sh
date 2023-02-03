#!/bin/sh

rm -rf test-ledger
cargo build
anchor build
solana-test-validator --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb /home/nidza/unique/blockchain/unq-spl/solana-program-library/target/deploy/spl_token_2022.so \
--bpf-program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA /home/nidza/unique/blockchain/unq-spl/solana-program-library/target/deploy/spl_token.so 
