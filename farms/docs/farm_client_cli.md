# Farm Client CLI

`solana-farm-client` is a client-side command-line tool for interacting with liquidity Pools, Farms, Vaults, and Funds. It can also print on-chain metadata and perform general operations with tokens, system accounts, and governance.
Under the hood, it leverages [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md), but alternatively, the same functionality can be achieved by using [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md) or by sending raw instructions.

All commands handled by `solana-farm-client` don't require Main Router admin or Fund manager privileges.

## General commands

    balance                              Print SOL balance
    transfer                             Transfer SOL to another wallet
    token-address                        Print associated token account address
    token-balance                        Print token balance
    token-close                          Close associated token account
    token-create                         Create associated token account
    token-data                           Print token account metadata
    token-supply                         Print token supply
    token-transfer                       Transfer tokens to another wallet
    oracle-price                         Print oracle price
    unwrap-sol                           Transfer WSOL back to SOL by closing ATA
    wrap-sol                             Transfer SOL to the associated WSOL account
    sync-token-balance                   Updates token balance of the account
    wallet-balances                      Print all token balances for the wallet
    protocols                            Print description and stats of all supported protocols

## Metadata commands

    get                                  Query specified object in blockchain and print
    get-all                              Query all objects of the given type and print
    get-ref                              Query specified object by reference address and print
    list-all                             Query all object names of the given type and print

## Liquidity Pool commands

    find-pools                           Find all Pools with tokens A and B
    find-pools-with-lp                   Find all Pools for the given LP token
    pool-price                           Print pool price
    swap                                 Swap tokens in the pool
    deposit-pool                         Add liquidity to the pool
    withdraw-pool                        Remove liquidity from the pool

## Farm commands

    find-farms-with-lp                   Find all Farms for the given LP token
    stake                                Stake LP tokens to the farm
    stake-balance                        Print user's stake balance in the farm
    unstake                              Unstake LP tokens from the farm
    harvest                              Harvest farm rewards

## Vault commands

    find-vaults                          Find all Vaults with tokens A and B
    find-vaults-with-vt                  Find all Vaults for the given VT token
    crank-vault                          Crank single vault
    crank-vaults                         Crank all vaults
    deposit-vault                        Add liquidity to the vault
    deposit-vault-locked                 Add locked liquidity to the vault
    withdraw-vault                       Remove liquidity from the vault
    withdraw-vault-unlocked              Remove unlocked liquidity from the vault
    vault-info                           Print vault stats
    vault-user-info                      Print user stats for the vault

## Fund commands

    find-funds                           Find all Funds with Vault names matching given pattern
    request-deposit-fund                 Request a new deposit to the fund
    cancel-deposit-fund                  Cancel pending deposit to the Fund
    request-withdrawal-fund              Request a new withdrawal from the Fund
    cancel-withdrawal-fund               Cancel pending withdrawal from the Fund
    fund-assets                          Print fund assets info
    fund-custodies                       Print all fund custodies
    fund-custody                         Print fund custody info
    fund-info                            Print fund stats
    fund-user-info                       Print user stats for the fund
    fund-user-requests                   Print user requests for the fund
    fund-vault                           Print fund vault info
    fund-vaults                          Print all fund vaults
    start-liquidation-fund               Start the Fund liquidation
    update-fund-assets-with-custodies    Update fund assets info based on all custodies
    update-fund-assets-with-custody      Update fund assets info based on custody holdings
    update-fund-assets-with-vault        Update fund assets info based on vault holdings
    update-fund-assets-with-vaults       Update fund assets info based on all vaults

## Governance commands

    get-address                               Get governance account address
    get-config                                Get governance config
    get-instruction                           Print stored instruction in the proposal
    custody-new                               Create new token custody account
    proposal-new                              Create a new proposal
    proposal-cancel                           Cancel the proposal
    proposal-state                            Get proposal state
    sign-off                                  Sign off the proposal
    signatory-add                             Add a signatory to the proposal
    signatory-remove                          Remove the signatory from the proposal
    tokens-deposit                            Deposit governing tokens
    tokens-withdraw                           Withdraw governing tokens
    vote-cast                                 Cast a vote on the proposal
    vote-finalize                             Finalize the vote on the proposal
    vote-relinquish                           Remove the vote from the proposal
    instruction-execute                       Execute the instruction in the proposal
    instruction-flag-error                    Mark the instruction as failed
    instruction-insert                        Add a new custom instruction to the proposal
    instruction-insert-deposit-pool           Add a new add liquidity to the pool instruction to the proposal
    instruction-insert-deposit-vault          Add a new add liquidity to the vault instruction to the proposal
    instruction-insert-harvest                Add a new harvest instruction to the proposal
    instruction-insert-program-upgrade        Add a new program upgrade instruction to the proposal
    instruction-insert-stake                  Add a new stake instruction to the proposal
    instruction-insert-swap                   Add a new swap instruction to the proposal
    instruction-insert-token-transfer         Add a new token transfer instruction to the proposal
    instruction-insert-unstake                Add a new unstake instruction to the proposal
    instruction-insert-withdraw-fees-vault    Add a new withdraw fees from the vault instruction to the proposal
    instruction-insert-withdraw-pool          Add a new remove liquidity from the pool instruction to the proposal
    instruction-insert-withdraw-vault         Add a new remove liquidity from the vault instruction to the proposal
    instruction-remove                        Remove the instruction from the proposal
    instruction-verify                        Verify custom instruction in the proposal
    instruction-verify-deposit-pool           Verify that instruction in the proposal is an add liquidity to the pool
    instruction-verify-deposit-vault          Verify that instruction in the proposal is an add liquidity to the vault
    instruction-verify-harvest                Verify that instruction in the proposal is a harvest
    instruction-verify-program-upgrade        Verify that instruction in the proposal is a program upgrade
    instruction-verify-stake                  Verify that instruction in the proposal is a stake
    instruction-verify-swap                   Verify that instruction in the proposal is a swap
    instruction-verify-token-transfer         Verify that instruction in the proposal is a token transfer
    instruction-verify-unstake                Verify that instruction in the proposal is an unstake
    instruction-verify-withdraw-fees-vault    Verify that instruction in the proposal is a withdraw fees from the vault
    instruction-verify-withdraw-pool          Verify that instruction in the proposal is a remove liquidity from the pool
    instruction-verify-withdraw-vault         Verify that instruction in the proposal is a remove liquidity from the vault
