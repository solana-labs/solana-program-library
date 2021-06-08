# SPL Token Lending program command line interface

A basic CLI for initializing lending markets and reserves for SPL Token Lending.
See https://spl.solana.com/token-lending for more details

## Deploy a lending program

1. Clone the project:
   ```shell
   > git clone https://github.com/solana-labs/solana-program-library.git
   ```

1. Go to the directory:
   ```shell
   > cd solana-program-library
   ```

1. Check out this branch:
   ```shell
   > git checkout lending/cli
   ```

1. Generate a keypair for yourself (optional if you already have one):
   ```shell
   > solana-keygen new

   Wrote new keypair to ~/.config/solana/id.json
   ================================================================================
   pubkey: BFa2aEPaqnMmqWeo1cBZYCaj9urU111r3UaYLRGrjJzs
   ================================================================================
   Save this seed phrase and your BIP39 passphrase to recover your new keypair:
   your seed words here never share them not even with your mom
   ================================================================================
   ```
   This pubkey will be the owner of the lending market that can add reserves to it.

1. Generate a keypair for the program:
   ```shell
   > solana-keygen new -o ./lending.json

   Wrote new keypair to ./lending.json
   ============================================================================
   pubkey: DG3VGXtLTxVjAFz5Ae2fmiSCiQ6DPBARofzJKRBMKZYn
   ============================================================================
   Save this seed phrase and your BIP39 passphrase to recover your new keypair:
   your seed words here never share them not even with your mom
   ============================================================================
   ```
   This pubkey will be your Program ID.

1. Open `./token-lending/program/src/lib.rs` in your editor. In the line
   ```rust
   solana_program::declare_id!("TokenLending11111111111111111111111111111111");
   ```
   replace `TokenLending11111111111111111111111111111111` with your Program ID, e.g.:
   ```rust
   solana_program::declare_id!("DG3VGXtLTxVjAFz5Ae2fmiSCiQ6DPBARofzJKRBMKZYn");
   ```

1. Build the program:
   ```shell
   > cargo build
   > cargo build-bpf
   ```

1. Build the CLI:
   ```shell
   > cargo build --bins
   ```

1. Prepare to deploy to devnet:
   ```shell
   > solana config set --url https://devnet.solana.com
   ```

1. Score yourself some sweet SOL:
   ```shell
   > solana airdrop 10
   ```
   You'll use this for transaction fees, rent for your program and related accounts, and initial reserve liquidity.

1. Deploy the program:
   ```shell
   > solana program deploy --program-id lending.json target/deploy/spl_token_lending.so
   ```

1. Get your pubkey:
   ```shell
   > solana address

   BFa2aEPaqnMmqWeo1cBZYCaj9urU111r3UaYLRGrjJzs
   ```

1. Create a lending market, using the pubkey from the previous step as the `owner`:
   ```shell
   > target/debug/spl-token-lending create-market \
         --owner BFa2aEPaqnMmqWeo1cBZYCaj9urU111r3UaYLRGrjJzs \
         --oracle 5mkqGkkWSaSk2NL9p4XptwEQu4d5jFTJiurbbzdqYexF

   Creating lending market 9X3NxthBgMkiJphbyrXbRRGPAFuKbiLYf78PnRhwekNP
   Signature: 3MyAUTNpmnZ2X6KRP39Drf9paGTU4o2AUKkkjecvGDn9HbXqaDviHxSDkDyBNjtZm2zpcmyM6zcNemFv4mEdQdRN
   ```
   Note the lending market pubkey (e.g. `9X3NxthBgMkiJphbyrXbRRGPAFuKbiLYf78PnRhwekNP`).

   Run `target/debug/spl-token-lending create-market --help` for more details and options.

1. Wrap some of your SOL as an SPL Token:
   ```shell
   > target/debug/spl-token wrap 2

   Wrapping 2 SOL into CLJwbKePBbuw5zEnhjjnCNoPdB7T33i9Vi1fPNxvqJkU
   ```
   Note the SPL Token account pubkey (e.g. `CLJwbKePBbuw5zEnhjjnCNoPdB7T33i9Vi1fPNxvqJkU`).

1. Add a SOL reserve to your market, using the pubkey from the previous step as the `source`:
   ```shell
   > target/debug/spl-token-lending add-reserve \
         --market 9X3NxthBgMkiJphbyrXbRRGPAFuKbiLYf78PnRhwekNP \
         --source CLJwbKePBbuw5zEnhjjnCNoPdB7T33i9Vi1fPNxvqJkU \
         --amount 1000000000 \
         --pyth-product 8yrQMUyJRnCJ72NWwMiPV9dNGw465Z8bKUvnUC8P5L6F \
         --pyth-price BdgHsXrH1mXqhdosXavYxZgX6bGqTdj5mh2sxDhF8bJy
   ```
   - `--market` is your lending market pubkey.
   - `--source` is your wrapped SOL SPL Token account.
   - `--amount` is the amount of SOL to deposit (in lamports).
   - `--pyth-product` and `--pyth-price` are SOL/USD oracle
     accounts [provided by Pyth](https://github.com/pyth-network).

   Run `target/debug/spl-token-lending add-reserve --help` for more details and options.