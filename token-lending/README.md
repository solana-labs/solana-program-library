# Token Lending program

A lending protocol for the Token program on the Solana blockchain inspired by Aave and Compound.

Full documentation is available at https://spl.solana.com/token-lending

Web3 bindings are available in the `./js` directory.

### On-chain programs

| Cluster | Program Address |
| --- | --- |
| Mainnet Beta | [`LendZqTs8gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi`](https://explorer.solana.com/address/LendZqTs7gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi) |
| Testnet | [`25MruiXVk27KoQPNKpMr44UDPHPCcSpRBfhXcgMDTwqQ`](https://explorer.solana.com/address/LendZqTs8gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi?cluster=testnet) |
| Devnet | [`25MruiXVk27KoQPNKpMr44UDPHPCcSpRBfhXcgMDTwqQ`](https://explorer.solana.com/address/LendZqTs8gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi?cluster=devnet) |

### Deploy a lending program

1. [Install the Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)

1. Install the Token and Token Lending CLIs:
   ```shell
   cargo install spl-token-cli
   cargo install spl-token-lending-cli
   ```
   
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
   solana_program::declare_id!("LendZqTs8gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi");
   ```
   replace the Program ID with yours, e.g.:
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
   You'll use this for transaction fees, rent for your program accounts, and initial reserve liquidity.

1. Deploy the program:
   ```shell
   solana program deploy \
     -k owner.json \
     --program-id lending.json \
     target/deploy/spl_token_lending.so
   ```
   If the deployment doesn't succeed, follow [this guide](https://docs.solana.com/cli/deploy-a-program#resuming-a-failed-deploy) to resume it.

1. Wrap some of your SOL as an SPL Token:
   ```shell
   spl-token --owner owner.json wrap 2

   # Wrapping 2 SOL into CsbAUDhZfPpkv8jCcV9PPQqfBkUVd5kntubhBLLgMLVF
   ```
   You'll use this for initial reserve liquidity. Note the SPL Token account pubkey (e.g. `CsbAUDhZfPpkv8jCcV9PPQqfBkUVd5kntubhBLLgMLVF`).

1. Use the [Token Lending CLI](./cli/README.md) to create a lending market and add reserves to it.