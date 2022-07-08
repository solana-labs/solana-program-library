# Farm Control CLI

`solana-farm-ctrl` is a command-line tool for on-chain data management, Funds, and Vaults control. It can also generate metadata for Funds, Vaults, and their liquidity tokens.
Under the hood, it leverages [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md), but alternatively, the same functionality can be achieved by executing instructions directly.

Most of the commands executed by `solana-farm-ctrl` require Main Router admin or Fund manager privileges. It is not intended to be used by the end-users. If multisig is enabled, different admins must execute the same command multiple times until the required number of signatures is collected.

## Reference Database commands

    init                                 Initialize Reference DB on-chain
    init-all                             Initialize Reference DB of all storage types on-chain
    drop                                 Drop on-chain Reference DB
    drop-all                             Drop on-chain Reference DB for all storage types
    print-pda-all                        Derive Reference DB addresses for all objects
    print-size                           Print Reference DB and specified object sizes
    print-size-all                       Print Reference DB and all object sizes

## Metadata commands

    get                                  Query specified object in blockchain and print
    get-all                              Query all objects of the given type and print
    get-ref                              Query specified object by reference address and print
    list-all                             Query all objects of the given type and print
    generate                             Generate json boilerplate for the specified object
    load                                 Load objects from file and send to blockchain
    load-all                             Same as "load"
    remove                               Remove specified object from blockchain
    remove-all                           Remove all objects of the given type from blockchain
    remove-all-with-file                 Remove all objects in the file from blockchain
    remove-ref                           Remove specified reference from blockchain

## Vault commands

    vault-init                           Initialize the Vault
    vault-crank                          Crank the Vault
    vault-disable-deposits               Disable deposits for the specified object
    vault-disable-withdrawals            Disable withdrawals for the specified object
    vault-enable-deposits                Enable deposits for the specified object
    vault-enable-withdrawals             Enable withdrawals for the specified object
    vault-get-info                       Print current stats for the Vault
    vault-get-admins                     Print current admin signers for the Vault
    vault-set-admins                     Set new admins for the Vault
    vault-set-external-fee               Set new external fee percent for the Vault
    vault-set-fee                        Set new fee percent for the Vault
    vault-set-min-crank-interval         Set new min crank interval in seconds for the Vault
    vault-shutdown                       Shutdown the Vault
    vault-withdraw-fees                  Withdraw collected fees from the Vault

## Fund commands

    fund-add-custody                     Add a new custody to the Fund
    fund-add-vault                       Add a new Vault to the Fund
    fund-approve-deposit                 Approve pending deposit to the Fund
    fund-approve-withdrawal              Approve pending withdrawal from the Fund
    fund-deny-deposit                    Deny pending deposit to the Fund
    fund-deny-withdrawal                 Deny pending withdrawal from the Fund
    fund-deposit-pool                    Add liquidity to the Pool in the Fund
    fund-deposit-vault                   Add liquidity to the Vault in the Fund
    fund-deposit-vault-locked            Add locked liquidity to the Vault in the Fund
    fund-disable-deposits                Disables deposits to the Fund
    fund-disable-withdrawals             Disables withdrawals from the Fund
    fund-get-admins                      Print current admin signers for the Fund
    fund-get-info                        Print current stats for the Fund
    fund-harvest                         Harvest rewards from the Farm in the Fund
    fund-init                            Initialize the Fund
    fund-lock-assets                     Moves assets from Deposit/Withdraw custody to the Fund
    fund-remove-custody                  Remove the custody from the Fund
    fund-remove-vault                    Remove the Vault from the Fund
    fund-set-admins                      Set new admins for the Fund
    fund-set-assets-tracking-config      Set a new assets tracking config for the Fund
    fund-set-deposit-schedule            Set a new deposit schedule for the Fund
    fund-set-manager                     Set a new manager for the Fund
    fund-set-withdrawal-schedule         Set a new withdrawal schedule for the Fund
    fund-stake                           Stake LP tokens to the Farm in the Fund
    fund-stop-liquidation                Stop the Fund liquidation
    fund-swap                            Swap tokens in the Fund
    fund-unlock-assets                   Releases assets from the Fund to Deposit/Withdraw custody
    fund-unstake                         Unstake LP tokens from the Farm in the Fund
    fund-update-assets-with-custodies    Update Fund assets info based on all custodies
    fund-update-assets-with-custody      Update Fund assets info based on custody holdings
    fund-update-assets-with-vault        Update Fund assets info based on Vault holdings
    fund-update-assets-with-vaults       Update Fund assets info based on all Vaults
    fund-withdraw-fees                   Withdraw collected fees from the Fund
    fund-withdraw-pool                   Remove liquidity from the Pool in the Fund
    fund-withdraw-vault                  Remove liquidity from the Vault in the Fund
    fund-withdraw-vault-unlocked         Remove unlocked liquidity from the Vault in the Fund

## Multisig commands

    get-admins                           Print current admin signers for the Main Router
    set-admins                           Set new admins for the Main Router
    program-get-admins                   Print current admin signers for the program
    program-set-admins                   Set new admin signers for the program
    program-set-single-authority         Set single upgrade authority for the program
    program-upgrade                      Upgrade the program from the data buffer

## Governance commands

    governance init                      Initialize a new DAO
