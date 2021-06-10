# SPL Token Lending program command line interface

A basic CLI for initializing lending markets and reserves for SPL Token Lending.
See https://spl.solana.com/token-lending for more details

## Deploy a lending program on Solana

1. [Install the Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)

1. Clone the SPL repo:
   ```shell
   git clone https://github.com/solana-labs/solana-program-library.git
   ```

1. Go to the new directory:
   ```shell
   cd solana-program-library
   ```

1. Generate a keypair for yourself:
   ```shell
   solana-keygen new -o owner.json

   # Wrote new keypair to owner.json
   # ================================================================================
   # pubkey: F5242QU7NC4jM8yjqqiv8pZRjL9K8229EBTXHBYtSAo5
   # ================================================================================
   # Save this seed phrase and your BIP39 passphrase to recover your new keypair:
   # your seed words here never share them not even with your mom
   # ================================================================================
   ```
   This pubkey will be the owner of the lending market that can add reserves to it.

1. Generate a keypair for the program:
   ```shell
   solana-keygen new -o lending.json

   # Wrote new keypair to lending.json
   # ============================================================================
   # pubkey: 25MruiXVk27KoQPNKpMr44UDPHPCcSpRBfhXcgMDTwqQ
   # ============================================================================
   # Save this seed phrase and your BIP39 passphrase to recover your new keypair:
   # your seed words here never share them not even with your mom
   # ============================================================================
   ```
   This pubkey will be your Program ID.

1. Open `./token-lending/program/src/lib.rs` in your editor. In the line
   ```rust
   solana_program::declare_id!("TokenLending11111111111111111111111111111111");
   ```
   replace `TokenLending11111111111111111111111111111111` with your Program ID, e.g.:
   ```rust
   solana_program::declare_id!("25MruiXVk27KoQPNKpMr44UDPHPCcSpRBfhXcgMDTwqQ");
   ```

1. Build the program binaries:
   ```shell
   cargo build
   cargo build-bpf
   ```

1. Prepare to deploy to devnet:
   ```shell
   solana config set --url https://api.devnet.solana.com
   ```

1. Score yourself some sweet SOL:
   ```shell
   solana airdrop -k owner.json 10
   ```
   You'll use this for transaction fees, rent for your program and related accounts, and initial reserve liquidity.

1. Deploy the program:
   ```shell
   solana program deploy -k owner.json --program-id lending.json target/deploy/spl_token_lending.so
   ```
   If the deployment doesn't succeed, follow [this guide](https://docs.solana.com/cli/deploy-a-program#resuming-a-failed-deploy) to resume it.

1. Get your pubkey:
   ```shell
   solana address -k owner.json

   # F5242QU7NC4jM8yjqqiv8pZRjL9K8229EBTXHBYtSAo5
   ```

1. Create a lending market, using the pubkey from the previous step as the `owner`:
   ```shell
   target/debug/spl-token-lending create-market \
     --fee-payer owner.json \
     --owner F5242QU7NC4jM8yjqqiv8pZRjL9K8229EBTXHBYtSAo5 \
     --oracle 5mkqGkkWSaSk2NL9p4XptwEQu4d5jFTJiurbbzdqYexF

   # Creating lending market CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN
   # Signature: 262NEkpPMiBiTq2DUXd3G3TkkRqFZf4e5ebojzYDkP7XVaSRANK1ir5Gk8zr8XLW6CG2xGzNFvEcUrbnENwenEwa
   ```
   Note the lending market pubkey (e.g. `CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN`).

   Run `target/debug/spl-token-lending create-market --help` for more details and options.

1. Wrap some of your SOL as an SPL Token:
   ```shell
   target/debug/spl-token --owner owner.json wrap 2

   # Wrapping 2 SOL into CsbAUDhZfPpkv8jCcV9PPQqfBkUVd5kntubhBLLgMLVF
   ```
   Note the SPL Token account pubkey (e.g. `CsbAUDhZfPpkv8jCcV9PPQqfBkUVd5kntubhBLLgMLVF`).

1. Add a SOL reserve to your market, using the pubkey from the previous step as the `source`:
   ```shell
   target/debug/spl-token-lending add-reserve \
     --fee-payer owner.json \
     --owner owner.json \
     --market CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN \
     --source CsbAUDhZfPpkv8jCcV9PPQqfBkUVd5kntubhBLLgMLVF \
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