#!/bin/sh

rm -rf test-ledger
cargo build
anchor build
solana-test-validator --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb ../../target/deploy/spl_token_2022.so \
--bpf-program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA ../../target/deploy/spl_token.so 
