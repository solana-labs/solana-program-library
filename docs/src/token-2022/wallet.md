---
title: Wallet Guide
---

This guide is meant for wallet developers who want to support Token-2022.

Since wallets have very different internals for managing token account state
and connections to blockchains, this guide will focus on the very specific changes
required, without only vague mentions of code design.

## Motivation

Wallet developers are accustomed to only including one token program used for
all tokens.

To properly support Token-2022, wallet developers must make code changes.

Important note: if you do not wish to support Token-2022, you do not need to do
anything. The wallet will not load Token-2022 accounts, and transactions created
by the wallet will fail loudly if using Token-2022 incorrectly.

Most likely, transactions will fail with `ProgramError::IncorrectProgramId`
when trying to target the Token program with Token-2022 accounts.

## Prerequisites

When testing locally, be sure to use at least `solana-test-validator` version
1.14.17, which includes the Token-2022 program by default. This comes bundled
with version 2.3.0 of the `spl-token` CLI, which also supports Token-2022.

## Setup

You'll need some Token-2022 tokens for testing. First, create a mint with an
extension. We'll use the "Mint Close Authority" extension:

```console
$ spl-token -ul create-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb --enable-close
Creating token E5SUrbnx7bMBp3bRdMWNCFS3FXp5VpvFDdNFp8rjrMLM under program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb

Address:  E5SUrbnx7bMBp3bRdMWNCFS3FXp5VpvFDdNFp8rjrMLM
Decimals:  9

Signature: 2dYhT1M3dHjbGd9GFCFPXmHMtjujXBGhM8b5wBkx3mtUptQa5U9jjRTWHCEmUQnv8XLt2x5BHdbDUkZpNJFqfJn1
```

The extension is important because it will test that your wallet properly handles
larger mint accounts.

Next, create an account for your test wallet:

```console
$ spl-token -ul create-account E5SUrbnx7bMBp3bRdMWNCFS3FXp5VpvFDdNFp8rjrMLM --owner <TEST_WALLET_ADDRESS> --fee-payer <FEE_PAYER_KEYPAIR>
Creating account 4L45ZpFS6dqTyLMofmQZ9yuTqYvQrfCJfWL2xAjd5WDW

Signature: 5Cjvvzid7w2tNZojrWVCmZ2MFiezxxnWgJHLJKkvJNByZU2sLN97y85CghxHwPaVf5d5pJAcDV9R4N1MNigAbBMN
```

With the `--owner` parameter, the new account is an associated token account,
which includes the "Immutable Owner" account extension. This way, you'll also
test larger token accounts.

Finally, mint some tokens:

```console
$ spl-token -ul mint E5SUrbnx7bMBp3bRdMWNCFS3FXp5VpvFDdNFp8rjrMLM 100000 4L45ZpFS6dqTyLMofmQZ9yuTqYvQrfCJfWL2xAjd5WDW
Minting 100000 tokens
  Token: E5SUrbnx7bMBp3bRdMWNCFS3FXp5VpvFDdNFp8rjrMLM
  Recipient: 4L45ZpFS6dqTyLMofmQZ9yuTqYvQrfCJfWL2xAjd5WDW

Signature: 43rsisVeLKjBCgLruwTFJXtGTBgwyfpLjwm44dY2YLHH9WJaazEvkyYGdq6omqs4thRfCS4G8z4KqzEGRP2xoMo9
```

It's also helpful for your test wallet to have some SOL, so be sure to transfer some:

```console
$ solana -ul transfer <TEST_WALLET_ADDRESS> 10 --allow-unfunded-recipient
Signature: 5A4MbdMTgGiV7hzLesKbzmrPSCvYPG15e1bg3d7dViqMaPbZrdJweKSuY1BQAfq245RMMYeGudxyKQYkgKoGT1Ui
```

Finally, you can save all of these accounts in a directory to be re-used for testing:

```console
$ mkdir test-accounts
$ solana -ul account --output-file test-accounts/token-account.json --output json 4L45ZpFS6dqTyLMofmQZ9yuTqYvQrfCJfWL2xAjd5WDW
... output truncated ...
$ solana -ul account --output-file test-accounts/mint.json --output json E5SUrbnx7bMBp3bRdMWNCFS3FXp5VpvFDdNFp8rjrMLM
... output truncated ...
$ solana -ul account --output-file test-accounts/wallet.json --output json <TEST_WALLET_ADDRESS>
```

This way, whenever you want to restart your test validator, you can simply run:

```console
$ solana-test-validator -r --account-dir test-accounts
```

## Structure of this Guide

We'll go through the required code changes to support Token-2022 in your wallet,
using only little code snippets. This work was done for the Backpack wallet in
[PR #3976](https://github.com/coral-xyz/backpack/pull/3976),
but as mentioned earlier, the actual code changes may look very different for
your wallet.

## Part I: Fetch Token-2022 Accounts

In addition to normal Token accounts, your wallet must also fetch Token-2022
accounts. Typically, wallets use the `getTokenAccountsByOwner` RPC endpoint once
to fetch the accounts.

For Token-2022, you simply need to add one more call to get the additional accounts:

```typescript
import { Connection, PublicKey } from '@solana/web3.js';

const TOKEN_PROGRAM_ID = new PublicKey(
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
);
const TOKEN_2022_PROGRAM_ID = new PublicKey(
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'
);
const walletPublicKey = new PublicKey('11111111111111111111111111111111'); // insert your key
const connection = new Connection('http://127.0.0.1:8899', 'confirmed');

const tokenAccounts = await connection.getTokenAccountsByOwner(
  walletPublicKey, { programId: TOKEN_PROGRAM_ID }
);
const token2022Accounts = await connection.getTokenAccountsByOwner(
  walletPublicKey, { programId: TOKEN_2022_PROGRAM_ID }
);
```

Merge the two responses, and you're good to go! If you can see your test account,
then you've done it correctly.

If there are issues, your wallet may be deserializing the token account too strictly,
so be sure to relax any restriction that the data size must be equal to 165 bytes.

## Part II: Use the Token Program Id for Instructions

If you try to transfer or burn a Token-2022 token, you will likely receive an
error because the wallet is trying to send an instruction to Token instead of
Token-2022.

Here are two possible ways to resolve the problem.

### Option 1: Store the token account's owner during fetch

In the first part, we fetched all of the token accounts and threw away the 
program id associated with the account. Instead of always targeting the Token
program, we need to target the right program for that token.

If we store the program id for each token account, then we can re-use that
information when we need to transfer or burn.

```typescript
import { Connection, PublicKey } from '@solana/web3.js';
import { createTransferInstruction } from '@solana/spl-token';

const TOKEN_PROGRAM_ID = new PublicKey(
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
);
const TOKEN_2022_PROGRAM_ID = new PublicKey(
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'
);
const walletPublicKey = new PublicKey('11111111111111111111111111111111'); // insert your key
const connection = new Connection('http://127.0.0.1:8899', 'confirmed');

const tokenAccounts = await connection.getTokenAccountsByOwner(
  walletPublicKey, { programId: TOKEN_PROGRAM_ID }
);
const token2022Accounts = await connection.getTokenAccountsByOwner(
  walletPublicKey, { programId: TOKEN_2022_PROGRAM_ID }
);
const accountsWithProgramId = [...tokenAccounts.value, ...token2022Accounts.value].map(
  ({ account, pubkey }) =>
    {
      account,
      pubkey,
      programId: account.data.program === 'spl-token' ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID,
    },
);

// later on...
const accountWithProgramId = accountsWithProgramId[0];
const instruction = createTransferInstruction(
  accountWithProgramId.pubkey,    // source
  accountWithProgramId.pubkey,    // destination
  walletPublicKey,                // owner
  1,                              // amount
  [],                             // multisigners
  accountWithProgramId.programId, // token program id
);
```

### Option 2: Fetch the program owner before transfer / burn

This approach introduces one more network call, but may be simpler to integrate.
Before creating an instruction, you can fetch the mint, source account, or
destination account from the network, and pull out its `owner` field.

```typescript
import { Connection, PublicKey } from '@solana/web3.js';

const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
const accountPublicKey = new PublicKey('11111111111111111111111111111111'); // insert your account key here
const accountInfo = await connection.getParsedAccountInfo(accountPublicKey);
if (accountInfo.value === null) {
    throw new Error('Account not found');
}
const programId = accountInfo.value.owner;
```

## Part III: Use the Token Program Id for Associated Token Accounts

Whenever we derive an associated token account, we must use the correct token
program id. Currently, most implementations hardcode the token program id.
Instead, you must add the program id as a parameter:

```typescript
import { PublicKey } from '@solana/web3.js';

const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);

function associatedTokenAccountAddress(
  mint: PublicKey,
  wallet: PublicKey,
  programId: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [wallet.toBuffer(), programId.toBuffer(), mint.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID
  )[0];
}
```

If you're creating associated token accounts, you'll also need to pass the
token program id, which currently defaults to `TOKEN_PROGRAM_ID`:

```typescript
import { Connection, PublicKey } from '@solana/web3.js';
import { createAssociatedTokenAccountInstruction } from '@solana/spl-token';

const tokenProgramId = new PublicKey(
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'
); // either `Tokenz...` or `Tokenkeg...`
const wallet = new PublicKey('11111111111111111111111111111111'); // insert your key
const mint = new PublicKey('11111111111111111111111111111111'); // insert mint key
const associatedTokenAccount = associatedTokenAccountAddress(mint, wallet, tokenProgramId);

const instruction = createAssociatedTokenAccountInstruction(
  wallet,                 // payer
  associatedTokenAccount, // associated token account
  wallet,                 // owner
  tokenProgramId,         // token program id
);
```

With these three parts done, your wallet will provide basic support for Token-2022!
