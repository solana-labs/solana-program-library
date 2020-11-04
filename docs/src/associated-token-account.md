---
title: Associated Token Account Program
---

This program defines the convention and the provides the mechanism for mapping
the user's wallet address to the associated token accounts they hold.

It also enables sender-funded token transfers.

See the [SPL Token](token.md) program for more information about tokens in
general.

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Associated Token Account Program's source is available on
[github](https://github.com/solana-labs/solana-program-library).


## Interface
The Associated Token Account Program is written in Rust and available on
[crates.io](https://crates.io/crates/spl-associated-token-account) and
[docs.rs](https://docs.rs/spl-associated-token-account).


### Finding the Associated Token Account address
The associated token account for a given wallet address is simply a
program-derived account consisting of the wallet address itself and the token mint.

The [get_associated_token_address](https://github.com/solana-labs/solana-program-library/blob/associated-token-account-v1.0.0/associated-token-account/program/src/lib.rs#L35)
Rust function may be used by clients to derive the wallet's associated token address.


The associated account address can be derived in Javascript with:
```js
import {PublicKey, PublicKeyNonce} from '@solana/web3.js';

const SPL_TOKEN_PROGRAM_ID: PublicKey = new PublicKey(
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
);
const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: PublicKey = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',
);

async function findAssociatedTokenAddress(
    walletAddress: Pubkey,
    tokenMintAddress: Pubkey
): Promise<PublicKey> {
    return PublicKey.findProgramAddress(
        [
            walletAddress.toBuffer(),
            SPL_TOKEN_PROGRAM_ID.toBuffer(),
            tokenMintAddress.toBuffer(),
        ],
        SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID
    )[0];
}
```


### Creating an Associated Token Account

If the associated token account for a given wallet address does not yet exist,
it may be created by *anybody* by issuing a transaction containing the
instruction return by [create_associated_token_account](https://github.com/solana-labs/solana-program-library/blob/associated-token-account-v1.0.0/associated-token-account/program/src/lib.rs#L54).

Regardless of creator the new associated token account will be fully owned by
the wallet, as if the wallet itself had created it.
