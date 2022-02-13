Token Lending program
A lending protocol for the Token program on the Solana blockchain inspired by Aave and Compound.

Full documentation is available at https://spl.solana.com/token-lending

Web3 bindings are available in the ./js directory.

Overview
The Token Lending Program allows the borrowing and supplying of liquidity of assets. Users may borrow liquidity by supplying a collateral amount or by using the Flash Loan option. The Token Lending Program takes ownership, with a program id and pointing to the appropriate token mint, of the tokens when certain functions are called, such as initializing an obligation or a lending market. The process of credit checking is not implemented, as the program uses collateral in exchange for liquidity.

Background
Solana's programming model and the definitions of the Solana terms used in this document are available at:

https://docs.solana.com/apps https://docs.solana.com/terminology

Operational Overview
The following sections explain the three main components in the Token Lending Program:

Lending markets
Lending market reserves
Obligations
Note that each instruction has a simple code example that can be found in the end-to-end tests. The program has 14 different instructions called at different times by the entrypoint.

Lending Markets
Users can initiate lending markets, set lending market owners, initialize reserves, and deposit liquidity to reserves. To initialize a lending market, process_init_lending_market is called with the owner's public key along with the program_id, quote-currency, and accounts-namely lending market, a rent sysvar, token program id, and oracle program id. The state is updated with a LendingMarket struct that contains version, bump_seed, owner public key, quote_currency, token_program_id public key, and oracle_program_id (provided by Pyth). The lending market's owner can be changed with the process_set_lending_market_owner with the parameters: program_id, new_owner, and accounts (lending market and lending market owner information).

Lending Market Reserves
Reserves are specified with the apprpriate SPL token mint and created with fn process_init_reserve( program_id: &Pubkey, liquidity_amount: u64, config: ReserveConfig, accounts: &[AccountInfo], ) in the entrypoint.

The program updates the state of the reserves with the struct Reserve that contains information about the version, last_update, lending_market public key, liquidity, collateral, and config configuration values. The fields last_update, liquidity, collateral, and config are all struct types, pointing to LastUpdate, ReserveLiquidity, ReserveCollateral, and ReserveConfig respectively. These structs have their methods implemented instate/state.rs. These methods include Redeeming collateral and Calculating borrow rates.

Collateral Tokens
When initializing a reserve, depositing liquidity grants users a share of the reserve liquidity pool with collateral tokens; the following process occurs after all accounts are verified:

Two accounts are initialized. One points to the reserve_liquidity_supply_info and the other to the reserve_liquidity_fee_receiver. Both of these accounts point to the same token mint, reserve_liquidity_mint_info.
A new mint is initialized and points to reserve_collateral_mint_info for the appropriate token mint, and sets the lending_market_authority_info.key as the authority for this instruction.
Another account is initialized, also owned by the lending_market_authority_info. This is for the reserve_collateral_supply_info.
One final account is created for the destination_collateral_info, with its own mint reserve_collateral_mint_info, and its owner pointing to the user_transfer_authority_info. The user transfer authority is the
Ultimately, a certain amount of tokens are minted to destination_collateral_info (account receiving collateral), all authorized by the lending_market_authority_info.
When supplying liquidity, the collateral token's lifecycle begins after all accounts have been verified and liquidity is deposited:

The liquidity from the supplier, source_liquidity_info is transferred to the appropriate reserve_liquidity_info, with the amounts and other values specified.
Then, the lending market, lending_market_authority_info, authorizes the creation of tokens in a specified amount, collateral_amount, to the supplier, destination_collateral_info.
Users may withdraw their liquidity using their collateral tokens, after accounts have been verified, in the following process:

A token burn occurs using the native spl_token_burn method, which specifies the token mint, reserve_collateral_info, the amount, collateral_amount, and is authorized by the user_transfer_authority.
The transfer is then requested, which originates from the liquidity reserve, reserve_liquidity_supply_info, and is directed to the destination_liquidity_info with the amount specified, liquidity_amount, authorized by the lending_market_authority_info`.

Obligations
Obligations are created with fn process_init_obligation fn process_init_obligation(program_id: &Pubkey, accounts: &[AccountInfo]). This processes the accounts in the slice accounts, making preliminary checks with the obligation account, including asserting the obligation account is rent-exempt, that the obligaiton is uninitialized, and that the Market Lending Program owns the obligation initially, respectively in that order.

The obligation states are updated in state/obligation.rs, which maintains pub struct Obligation. This object contains values, along with two additional structs, ObligationCollateral and ObligationLiquidity. The former tracks deposited collateral with the appropriate public key that points to the deposit_reserve; this struct may also increase and decrease collateral . The latter also uses a public key that [points](also increase and decrease collateral) to the address of its borrow reserve.

Flash Loans
Users can borrow liquidity without collateral, and with no risk to the liquidity reserves. Flash loans must be paid within the same transaction block with a fee. The failsafe involves the nature of atomic transactions; if the borrower does not pay back within the same block, their liquidity is yanked back to the source- all involved states are reverted, and the borrower bears all the risk with his gas payment.

This mechanism is implemented with several accounts involved, such as the host fee receiver. After the accounts are verified and the state of reserves is updated, an spl_token_transfer command sends a specified amount of liquidity to the borrower. Furthermore, a native invoke() command is called that specifies the program invocation, accounts, and data within the Instruction; flash_loan_instruction_account_infos are the accounts required in the invocaiton. The state is changed again, this time with the .repay() method. The state can be absolutely reverted after the balance of the liquidity pool is less than expected, returning a LendingError::NotEnoughLiquidityAfterFlashLoan error. Ultimately, the fees are paid to the host, then to the owner if there is an owner_fee, by the liquidity pool.

Market Pricing
Pyth pricing structs, Product and Price, are imported and configured in pyth.rs; pricing is then extracted in processor.rs, using the methods get_pyth_product_quote_currency and get_pyth_price. Getting a quote currency using the former method involves taking a &pyth::Product parameter, which uses the attr value to return a 32-byte length array. This is extracted when initializing a lending market reserve, which uses this byte array to check against the lending market account's quote currency.

Product prices are checked for novelty against a variable STALE_AFTER_SLOTS_ELAPSED. The pricing of a product is considered stale after five slots have passed. This is done by using the clock sysvar with native methods that check the current slot at the time of asking for product pricing, similar to time stamping using blocks in Ethereum. Furthermore, refreshing prices of reserve liquidity is essential and is handled with process_refresh_reserve. After a mut reserve is unpacked and instantiated, the reserve has its market_price value updated using Pyth.

On-chain programs
Please note that only the lending program deployed to devnet is currently operational.

Cluster	Program Address
Mainnet Beta	LendZqTs8gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi
Testnet	6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH
Devnet	6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH
Documentation
CLI docs
Client library docs
Deploy a lending program (optional)
This is optional! You can skip these steps and use the Token Lending CLI with one of the on-chain programs listed above to create a lending market and add reserves to it.

Install the Solana CLI

Install the Token and Token Lending CLIs:

cargo install spl-token-cli
cargo install spl-token-lending-cli
Clone the SPL repo:

git clone https://github.com/solana-labs/solana-program-library.git
Go to the new directory:

cd solana-program-library
Generate a keypair for yourself:

solana-keygen new -o owner.json

# Wrote new keypair to owner.json
# ================================================================================
# pubkey: JAgN4SZLNeCo9KTnr8EWt4FzEV1UDgHkcZwkVtWtfp6P
# ================================================================================
# Save this seed phrase and your BIP39 passphrase to recover your new keypair:
# your seed words here never share them not even with your mom
# ================================================================================
This pubkey will be the owner of the lending market that can add reserves to it.

Generate a keypair for the program:

solana-keygen new -o lending.json

# Wrote new keypair to lending.json
# ============================================================================
# pubkey: 6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH
# ============================================================================
# Save this seed phrase and your BIP39 passphrase to recover your new keypair:
# your seed words here never share them not even with your mom
# ============================================================================
This pubkey will be your Program ID.

Open ./token-lending/program/src/lib.rs in your editor. In the line

solana_program::declare_id!("6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH");
replace the Program ID with yours.

Build the program binaries:

cargo build
cargo build-bpf
Prepare to deploy to devnet:

solana config set --url https://api.devnet.solana.com
Score yourself some sweet SOL:

solana airdrop -k owner.json 10
solana airdrop -k owner.json 10
solana airdrop -k owner.json 10
You'll use this for transaction fees, rent for your program accounts, and initial reserve liquidity.

Deploy the program:

solana program deploy \
  -k owner.json \
  --program-id lending.json \
  target/deploy/spl_token_lending.so

# Program Id: 6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH
If the deployment doesn't succeed, follow this guide to resume it.

Wrap some of your SOL as an SPL Token:

spl-token wrap \
   --fee-payer owner.json \
   10.0 \
   -- owner.json

# Wrapping 10 SOL into AJ2sgpgj6ZeQazPPiDyTYqN9vbj58QMaZQykB9Sr6XY
You'll use this for initial reserve liquidity. Note the SPL Token account pubkey (e.g. AJ2sgpgj6ZeQazPPiDyTYqN9vbj58QMaZQykB9Sr6XY).

Use the Token Lending CLI to create a lending market and add reserves to it.