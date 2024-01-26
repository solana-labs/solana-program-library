---
title: Configuring Extra Accounts
---

As mentioned previously, programs who implement the Transfer Hook interface can
provide additional custom functionality to token transfers. However, this
functionality may require additional accounts beyond those that exist in a
transfer instruction (source, mint, destination, etc.).

Part of the Transfer Hook interface specification is the validation account - an
account which stores configurations for additional accounts required by the
transfer hook program.

### The Validation Account

The validation account is a PDA off of the transfer hook program derived from
the following seeds:

```
"extra-account-metas" + <mint-address>
```

As you can see, one validation account maps to one mint account. This means you
can customize the additional required accounts on a per-mint basis!

The validation account stores configurations for extra accounts using
[Type-Length-Value](https://en.wikipedia.org/wiki/Type%E2%80%93length%E2%80%93value)
(TLV) encoding:
- **Type:** The instruction discriminator, in this case `Execute`
- **Length:** The total length of the subsequent data buffer, in this case a
  `u32`
- **Data:** The data itself, in this case containing the extra account
  configurations

When a transfer hook program seeks to deserialize extra account configurations
from a validation account, it can find the 8-byte instruction discriminator for
`Execute`, then read the length, then use that length to deserialize the data.

The data itself is a list of fixed-size configuration objects serialized into a
byte slab. Because the entries are fixed-length, we can use a custom "slice"
structure which divides the length by the fixed-length to determine the number
of entries.

This custom slice structure is called a `PodSlice` and is part of the Solana
Program Library's
[Pod](https://github.com/solana-labs/solana-program-library/tree/master/libraries/pod)
library. The Pod library provides a handful of fixed-length types that
implement the `bytemuck`
[`Pod`](https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html) trait, as well
as the `PodSlice`.

Another SPL library
useful for Type-Length-Value encoded data is
[Type-Length-Value](https://github.com/solana-labs/solana-program-library/tree/master/libraries/type-length-value)
which is used extensively to manage TLV-encoded data structures.

### Dynamic Account Resolution

When clients build a transfer instruction to the token program, they must
ensure the instruction includes all required accounts, especially the extra
required accounts you've specified in the validation account.

These additional accounts must be _resolved_, and another library used to pull off
the resolution of additional accounts for transfer hooks is
[TLV Account Resolution](https://github.com/solana-labs/solana-program-library/tree/master/libraries/tlv-account-resolution).

Using the TLV Account Resolution library, transfer hook programs can empower
**dynamic account resolution** of additional required accounts. This means that
no particular client or program needs to know the specific accounts your
transfer hook requires. Instead, they can be automatically resolved from the
validation account's data.

In fact, the Transfer Hook interface offers helpers that perform this account
resolution in the
[onchain](https://github.com/solana-labs/solana-program-library/blob/master/token/transfer-hook/interface/src/onchain.rs)
and
[offchain](https://github.com/solana-labs/solana-program-library/blob/master/token/transfer-hook/interface/src/offchain.rs)
modules of the Transfer Hook interface crate.

The account resolution is powered by the way configurations for additional
accounts are stored, and how they can be used to derive actual Solana addresses
and roles (signer, writeable, etc.) for accounts.

### The `ExtraAccountMeta` Struct

A member of the TLV Account Resolution library, the
[`ExtraAccountMeta`](https://github.com/solana-labs/solana-program-library/blob/65a92e6e0a4346920582d9b3893cacafd85bb017/libraries/tlv-account-resolution/src/account.rs#L75)
struct allows account configurations to be serialized into a fixed-length data
format of length 35 bytes.

```rust
pub struct ExtraAccountMeta {
    /// Discriminator to tell whether this represents a standard
    /// `AccountMeta` or a PDA
    pub discriminator: u8,
    /// This `address_config` field can either be the pubkey of the account
    /// or the seeds used to derive the pubkey from provided inputs
    pub address_config: [u8; 32],
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}
```

As the documentation on the struct conveys, an `ExtraAccountMeta` can store
configurations for three types of accounts:

|Discriminator|Account Type|
|:------------|:-----------|
|`0` | An account with a static address |
| `1` | A PDA off of the transfer hook program itself |
| `(1 << 7) + i ` | A PDA off of another program, where `i` is that program's index in the accounts list |

`1 << 7` is the top bit of the `u8`, or `128`. If the program you are deriving
this PDA from is at index `9` of the accounts list for `Execute`, then the
discriminator for this account configuration is `128 + 9 = 137`. More on
determining this index later.

#### Accounts With Static Addresses

Static-address additional accounts are straightforward to serialize with
`ExtraAccountMeta`. The discriminator is simply `0` and the `address_config` is
the 32-byte public key.

#### PDAs Off the Transfer Hook Program

You might be wondering: "how can I store all of my PDA seeds in only 32 bytes?".
Well, you don't. Instead, you tell the account resolution functionality _where_
to find the seeds you need.

To do this, the transfer hook program can use the
[`Seed`](https://github.com/solana-labs/solana-program-library/blob/65a92e6e0a4346920582d9b3893cacafd85bb017/libraries/tlv-account-resolution/src/seeds.rs#L38)
enum to describe their seeds and where to find them. With the exception of
literals, these seed configurations comprise only a small handful of bytes.

The following types of seeds are supported by the `Seed` enum and can be used to
create an `address_config` array of bytes.
- **Literal**: The literal seed itself encoded to bytes
- **Instruction Data:** A slice of the instruction data, denoted by the `index`
  (offset) and `length` of bytes to slice
- **AccountKey:** The address of some account in the list as bytes, denoted by
  the `index` at which this account can be found in the accounts list
- **Account Data:** A slice of an account's data, denoted by the `account_index`
  at which this account can be found in the accounts list, as well as the
  `data_index` (offset) and `length` of bytes to slice

Here's an example of packing a list of `Seed` entries into a 32-byte
`address_config`:

```rust
let seed1 = Seed::Literal { bytes: vec![1; 8] };
let seed2 = Seed::InstructionData {
    index: 0,
    length: 4,
};
let seed3 = Seed::AccountKey { index: 0 };
let address_config: [u8; 32] = Seed::pack_into_address_config(
  &[seed1, seed2, seed3]
)?;
```

#### PDAs Off Another Program

Storing configurations for seeds for an address that is a PDA off of another
program is the same as above. However, the program whose address this account is
a PDA off of must be present in the account list. Its index in the accounts list
is required to build the proper discriminator, and thus resolve the proper PDA.

```rust
let program_index = 7;
let seeds = &[seed1, seed2, seed3];
let is_signer = false;
let is_writable = true;

let extra_meta = ExtraAccountMeta::new_external_pda_with_seeds(
  program_index,
  seeds,
  is_signer,
  is_writable,
)?;
```

