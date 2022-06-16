#!/usr/bin/env bash
args=(
  --reset
  --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s ../../../deps/metaplex-program-library/token-metadata/target/deploy/mpl_token_metadata.so
  --bpf-program BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY ../../target/deploy/bubblegum.so
  --bpf-program Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS ../../target/deploy/gummyroll_crud.so
  --bpf-program GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD ../../target/deploy/gummyroll.so
  --bpf-program BRKyVDRGT7SPBtMhjHN4PVSPVYoc3Wa3QTyuRVM4iZkt ../../target/deploy/gumball_machine.so
  --bpf-program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA ../../../deps/solana-program-library/target/deploy/spl_token.so
  --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb ../../../deps/solana-program-library/target/deploy/spl_token_2022.so
  --bpf-program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL ../../../deps/solana-program-library/target/deploy/spl_associated_token_account.so
)
echo "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
solana-test-validator "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
