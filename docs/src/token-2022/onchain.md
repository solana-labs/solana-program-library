---
title: On-chain Program Guide
---

## Supporting Token and Token-2022 Together In Your Program

This guide is meant for on-chain program / dapp developers who want to support
Token and Token-2022 concurrently.

## Prerequisites

This guide requires the Solana CLI tool suite, minimum version 1.10.33 in order
to support all Token-2022 features.

## Motivation

On-chain program developers are accustomed to only including one token program,
to be used for all tokens in the application.

With the addition of Token-2022, developers must update on-chain programs. This
guide walks through the steps required to support both.

Important note: if you do not wish to support Token-2022, there is nothing to do.
Your existing on-chain program will loudly fail if an instruction includes any
Token-2022 mints / accounts.

Most likely, your program will fail with `ProgramError::IncorrectProgramId` while
trying to create a CPI instruction into the Token program, providing the Token-2022
program id.

## Structure of this Guide

To safely code the transition, we'll follow a test-driven development approach:

* add a dependency to `spl-token-2022`
* change tests to use `spl_token::id()` or `spl_token_2022::id()`, see that all
tests fail with Token-2022
* update on-chain program code to always use the instruction and deserializers from 
`spl_token_2022`, make all tests pass

Optionally, if an instruction uses more than one token mint, common to most DeFi,
you must add an input token program account for each additional mint. Since it's
possible to swap all types of tokens, we need to either invoke the correct token
program.

Everything here will reference real commits to the token-swap program, so
feel free to follow along and make the changes to your program.

## Part I: Support both token programs in single-token use cases

### Step 1: Update dependencies

In your `Cargo.toml`, add the latest `spl-token-2022` to your `dependencies`.
Check for the latest version of `spl-token-2022` in [crates.io](https://crates.io), since
that will typically be the version deployed to mainnet-beta.

### Step 2: Add test cases for Token and Token-2022

Using the `test-case` crate, you can update all tests to use both Token and
Token-2022. For example, a test defined as:

```
#[tokio::test]
async fn test_swap() {
    ...
}
```

Will become:

```
#[test_case(spl_token::id() ; "Token Program")]
#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
#[tokio::test]
async fn test_swap(token_program_id: Pubkey) {
    ...
}
```

In your program-test setup, you must include `spl_token_2022.so` at the correct
address. You can add it as normal to `tests/fixtures/` after downloading it using:

```console
$ solana program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb spl_token_2022.so
```

If you're using `solana-test-validator` for your tests, you can include it using:

```console
$ solana-test-validator -c TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb 
```

**Note**: This step is temporary, until Token-2022 is included by default in
`program-test` and `solana-test-validator`.

The token-swap does not use `program-test`, so there's a bit more
boilerplate, but the same principle applies.

### Step 3: Replace instruction creators

Everywhere in the code that uses `spl_token::instruction` must now use
`spl_token_2022::instruction`. The `"Token-2022 Program"` tests will still fail,
but importantly, the `"Token Program"` tests will pass using the new instruction
creators.

If your program uses unchecked transfers, you'll see a deprecation warning:

```
warning: use of deprecated function `spl_token_2022::instruction::transfer`: please use `transfer_checked` or `transfer_checked_with_fee` instead
```

If a token has a transfer fee, an unchecked transfer will fail. We'll fix that
later. If you want, in the meantime, feel free to add an `#[allow(deprecated)]`
to pass CI, with a TODO or issue to transition to `transfer_checked` everywhere.

### Step 4: Replace spl_token::id() with a parameter

Step 2 started the transition away from a fixed program id by adding
`token_program_id` as a parameter to the test function, but now you'll go
through your program and tests to use it everywhere.

Whenever `spl_token::id()` appears in the code, use a parameter corresponding
either to `spl_token::id()` or `spl_token_2022::id()`.

After this, all of your tests should pass! Not so fast though, there's one more
step needed to ensure compatibility.

### Step 5: Add Extensions to Tests

Although all of your tests are passing, you still need to account for differences
in accounts in token-2022.

Account extensions are stored after the first 165 bytes of the account, and the
normal `Account::unpack` and `Mint::unpack` will fail if the size of the account
is not exactly 165 and 82, respectively.

Let's make the tests fail again by adding an extension to all mint and token
accounts.  We'll add the `MintCloseAuthority` extension to mints, and the `ImmutableOwner`
extension to accounts.

When creating mint accounts, calculate the space required before allocating, then
include an `initialize_mint_close_authority` instruction before `initialize_mint`.
For example this could be:

```rust
use spl_token_2022::{extension::ExtensionType, instruction::*, state::Mint};
use solana_sdk::{system_instruction, transaction::Transaction};

// Calculate the space required using the `ExtensionType`
let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);

// get the Rent object and calculate the rent required
let rent_required = rent.minimum_balance(space);

// and then create the account using those parameters
let create_instruction = system_instruction::create_account(&payer.pubkey(), mint_pubkey, rent_required, space, token_program_id);

// Important: you must initialize the mint close authority *BEFORE* initializing the mint,
// and only when working with Token-2022, since the instruction is unsupported by Token.
let initialize_close_authority_instruction = initialize_mint_close_authority(token_program_id, mint_pubkey, Some(close_authority)).unwrap();
let initialize_mint_instruction = initialize_mint(token_program_id, mint_pubkey, mint_authority_pubkey, freeze_authority, 9).unwrap();

// Make the transaction with all of these instructions
let create_mint_transaction = Transaction::new(&[create_instruction, initialize_close_authority_instruction, initialize_mint_instruction], Some(&payer.pubkey));

// Sign it and send it however you want!
```

The concept is similar with token accounts, but we'll use the `ImmutableOwner`
extension, which is actually supported by both programs, but `Tokenkeg...` will
no-op.

```rust
use spl_token_2022::{extension::ExtensionType, instruction::*, state::Account};
use solana_sdk::{system_instruction, transaction::Transaction};

// Calculate the space required using the `ExtensionType`
let space = ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner]);

// get the Rent object and calculate the rent required
let rent_required = rent.minimum_balance(space);

// and then create the account using those parameters
let create_instruction = system_instruction::create_account(&payer.pubkey(), account_pubkey, rent_required, space, token_program_id);

// Important: you must initialize immutable owner *BEFORE* initializing the account
let initialize_immutable_owner_instruction = initialize_immutable_owner(token_program_id, account_pubkey).unwrap();
let initialize_account_instruction = initialize_account(token_program_id, account_pubkey, mint_pubkey, owner_pubkey).unwrap();

// Make the transaction with all of these instructions
let create_account_transaction = Transaction::new(&[create_instruction, initialize_immutable_owner_instruction, initialize_account_instruction], Some(&payer.pubkey));

// Sign it and send it however you want!
```

After making these changes, everything fails again. Well done!

### Step 6: Use `StateWithExtensions` instead of `Mint` and `Account`

The test failures happen because the program is trying to deserialize a pure
`Mint` or `Account`, and failing because there are extensions added to it.

Token-2022 adds a new type called `StateWithExtensions`, which allows you to
deserialize the base type, and then pull out any extensions on the fly. It's
very close to the same cost as the normal `unpack`.

Everywhere in your code, wherever you see `Mint::unpack` or `Account::unpack`,
you'll have to change that to:

```rust
use spl_token_2022::{extension::StateWithExtensions, state::{Account, Mint}};
let account_state = StateWithExtensions::<Account>::unpack(&token_account_info.data.borrow())?;
let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account_info.data.borrow())?;
```

Anytime you access fields in the state, you'll need to go through the `base`. For
example, to access the amount, you must do:

```rust
let token_amount = account_state.base.amount;
```

So typically, you'll just need to add in `.base` wherever those fields are accessed.

Once that's done, all of your tests should pass! Congratulations, your program
is now compatible with Token-2022!

If your program is using multiple token types at once, however, you will need to
do more work.

## Part II: Support Mixed Token Programs: trading a Token for a Token-2022

## Part III: Support Specific Extensions

### Update from `transfer` to `transfer_checked`

### Take fee into account when calculating slippage
