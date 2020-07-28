---
title: Token Program
---

A Fungible Token program on the Solana blockchain.

This program provides an interface and implementation that third parties can
utilize to create and use their tokens.

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:
- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Token Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

## Interface

The on-chain Token Program is written in Rust and available on crates.io as
[spl-token](https://docs.rs/spl-token). The program's [instruction interface
documentation](https://docs.rs/spl-token/1.0.2/spl_token/instruction/enum.TokenInstruction.html)
can also be found there.

Auto-generated C bindings are also available for the on-chain Token Program and
available
[here](https://github.com/solana-labs/solana-program-library/blob/master/token/inc/token.h)

[Javascript
bindings](https://github.com/solana-labs/solana-program-library/blob/master/token/js/client/token.js)
are available that support loading the Token Program on to a chain and issuing
instructions.

## Operational overview

### Creating a new token type

A new token type can be created by initializing a new Mint with the
`InitializeMint` instruction.  The Mint is used to create or "Mint" new tokens,
and these tokens are stored in Accounts.  A Mint is associated with each
Account, which means that the total supply of a particular token type is equal
to the balances of all the associated Accounts.

A Mint can either be configured with a fixed-supply or non-fixed supply.  The
total supply of a fixed-supply Mint is determined during initialization and
deposited into a provided destination account.  A non-fixed-supply Mint also has
an owner associated with it who has the authority to create new tokens in the
future with the `MintTo` instruction.  Both types of Mints can `Burn` tokens to
decrease supply.ß

It's important to note that the `InitializeMint` instruction does not require
the Solana account being initialized also be a signer.  The `InitializeMint`
instruction should be atomically processed with the system instruction that
creates the Solana account by including both instructions in the same
transaction.

### Creating accounts

Accounts hold token balances and are created using the `InitializeAccount`
instruction.  Each Account has an owner who must be present as a signer in some
instructions.

Balances can be transferred between Accounts using the `Transfer` instruction.
The owner of the source Account must be present as a signer in the `Transfer`
instruction.

An Account's owner may transfer ownership of an account to another using the
`SetOwner` instruction.

It's important to note that the `InitializeAccount` instruction does not require
the Solana account being initialized also be a signer.  The `InitializeAccount`
instruction should be atomically processed with the system instruction that
creates the Solana account by including both instructions in the same
transaction.

### Burning

The `Burn` instruction decreases an Account's token balance without transferring
to another Account, effectively removing the token from circulation permanently.

### Authority delegation

Account owners may delegate authority over some or all of their token balance
using the `Approve` instruction.  Delegated authorities may transfer or burn up
to the amount they've been delegated.  Authority delegation may be revoked by
the Account's owner via the `Revoke` instruction.

### Multisignatures

M of N multisignatures are supported and can be used in place of Mint owners, or
Account owners or delegates.  Multisignature owners or delegates must be
initialized with the `InitializeMultisig` instruction. Initialization specifies
the set of N public keys that are valid and the number M of those N that must be
present as instruction signers for the authority to be legitimate.

It's important to note that the `InitializeMultisig` instruction does not
require the Solana account being initialized also be a signer.  The
`InitializeMultisig` instruction should be atomically processed with the system
instruction that creates the Solana account by including both instructions in
the same transaction.

### Wrapping SOL

The Token Program can be used to wrap native SOL.  Doing so allows native SOL to
be treated like any other Token program token type and can be useful when being
called from other programs that interact with the Token Program's interface.

Accounts containing wrapped SOL are associated with a specific Mint called the
"Native Mint"  using the public key
`So11111111111111111111111111111111111111111`.  

These accounts have a few unique behaviors
- `InitializeAccount` sets the balance of the initialized Account to the SOL
  balance of the Solana account being initialized, resulting in a token balance
  equal to the SOL balance.
- Transfers to and from not only modify the token balance but also transfer an
  equal amount of SOL from the source account to the destination account.
- Burning is not supported
- When closing an Account the balance may be non-zero.

### Closing accounts

An account may be closed using the `CloseAccount` instruction.  When closing an
Account, all remaining SOL will be transferred to another Solana account
(doesn't have to be associated with the Token Program).  Non-native accounts
must have a balance of zero to be closed.