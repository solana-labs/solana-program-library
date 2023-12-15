# Developers Notes

## On pub enum GovernanceAccountType

### Asset-specific governances are deprecated 

The asset specific governances `ProgramGovernance, MintGovernance and TokenGovernance` are legacy and come from the time when they were top level primitives (Realms didn’t exist) and hence we wanted stronger guarantees to create governances for the relevant assets. They are deprecated now.

The recommendation is to use the generic governance account (`governanceV2`) to manage all kinds of assets.

### DAO Wallet

Every Governance account (which holds the governance rules) has an associated native SOL treasury (we call it DAO wallet). The relationship is always 1:1. Since the Governance account is a PDA with data it can’t be used as transaction payer and to store SOL in general. **To control programs, the recommended way is to always use the associated SOL address (DAO wallet) for authority over assets**

A DAO wallet is 1) PDA with no data, 2) derived from its governance account and 3) owned by System program. This way it behaves like any other wallet. 

If the intention is to manage a program, you should use the DAO wallet as the admin auth in your program

*Note: as of 2022-09-17 the UI is not up-to-date and it is still allowing users to create the deprecated asset specific governances.*

### Signing transactions: Use the DAO Wallet

Right now both PDAs (DAO Wallet and governance account) can sign Txs. However some protocols assume the signer is also a payer or beneficiary and then only the DAO wallet can be used. For that reason it’s always better to use the DAO wallet as the authority because it behaves like any other wallet and works for all scenarios. The objective is to standardize on DAO Wallet as signer to eliminate confusion.

### Wallet assets

We have the concept of wallet assets. An asset can be anything a DAO wallet can own (have authority over). This way you could define your programs as assets and show them in the wallet with your contextual actions. 
