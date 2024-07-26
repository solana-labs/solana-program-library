---
title: Specification
---

The Transfer Hook interface specification includes two optional instructions and
one required one.

Each instruction of the Transfer Hook interface uses a specific 8-byte
discriminator at the start of its instruction data.

### Instruction: `Execute`

The `Execute` instruction is required by any program who wishes to implement the
interface, and this is the instruction in which custom transfer functionality
will live.

- **Discriminator:** First 8 bytes of the hash of the string literal  
  `"spl-transfer-hook-interface:execute"`
- **Data:**
  - `amount: u64` - The transfer amount
- **Accounts:**
  - 1 `[]`: Source token account
  - 2 `[]`: Mint
  - 3 `[]`: Destination token account
  - 4 `[]`: Source token account authority
  - 5 `[]`: Validation account
  - `n` number of additional accounts, written into the validation account

The **validation account** is a key piece of the Transfer Hook interface, and is
covered in more detail in the [next section](./configuring-extra-accounts). In
short, it's an account whose data stores configurations that can be deserialized
to determine which additional accounts are required by the transfer hook
program.

The next two instructions of the interface deal with these configurations.

### (Optional) Instruction: `InitializeExtraAccountMetaList`

This instruction does exactly what the name implies: it initializes the
validation account to store a list of extra required
[`AccountMeta`](https://docs.rs/solana-program/latest/solana_program/instruction/struct.AccountMeta.html)
configurations for the `Execute` instruction.

- **Discriminator:** First 8 bytes of the hash of the string literal  
  `"spl-transfer-hook-interface:initialize-extra-account-metas"`
- **Data:**
  - `extra_account_metas: Vec<ExtraAccountMeta>` - A list of extra account
    configurations to be written into the validation account
- **Accounts:**
  - 1 `[writable]`: Validation account
  - 2 `[]`: Mint
  - 3 `[signer]`: Mint authority
  - 4 `[]`: System program

### (Optional) Instruction: `UpdateExtraAccountMetaList`

The `UpdateExtraAccountMetaList` instruction allows an on-chain program to
update its list of required accounts for `Execute`. By implementing this
instruction, developers can make updates to their list of required extra
accounts stored in the validation account.

- **Discriminator:** First 8 bytes of the hash of the string literal
  `"spl-transfer-hook-interface:update-extra-account-metas"`
- **Data:**
  - `extra_account_metas: Vec<ExtraAccountMeta>` - A list of extra account 
    configurations to be written into the validation account
- **Accounts:**
  - 1 `[writable]`: Validation account
  - 2 `[]`: Mint
  - 3 `[signer]`: Mint authority

