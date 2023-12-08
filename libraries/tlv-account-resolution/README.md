# TLV Account Resolution

Library defining a generic state interface to encode additional required accounts
for an instruction, using Type-Length-Value structures.

## Example usage

If you want to encode the additional required accounts for your instruction
into a TLV entry in an account, you can do the following:

```rust
use {
    solana_program::{account_info::AccountInfo, instruction::{AccountMeta, Instruction}, pubkey::Pubkey},
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_tlv_account_resolution::{
        account::ExtraAccountMeta,
        seeds::Seed,
        state::ExtraAccountMetaList
    },
};

struct MyInstruction;
impl SplDiscriminate for MyInstruction {
    // For ease of use, give it the same discriminator as its instruction definition
    const SPL_DISCRIMINATOR: ArrayDiscriminator = ArrayDiscriminator::new([1; ArrayDiscriminator::LENGTH]);
}

// Prepare the additional required account keys and signer / writable
let extra_metas = [
    AccountMeta::new(Pubkey::new_unique(), false).into(),
    AccountMeta::new_readonly(Pubkey::new_unique(), true).into(),
    ExtraAccountMeta::new_with_seeds(
        &[
            Seed::Literal {
                bytes: b"some_string".to_vec(),
            },
            Seed::InstructionData {
                index: 1,
                length: 1, // u8
            },
            Seed::AccountKey { index: 1 },
        ],
        false,
        true,
    ).unwrap(),
    ExtraAccountMeta::new_external_pda_with_seeds(
        0,
        &[Seed::AccountKey { index: 2 }],
        false,
        false,
    ).unwrap(),
];

// Allocate a new buffer with the proper `account_size`
let account_size = ExtraAccountMetaList::size_of(extra_metas.len()).unwrap();
let mut buffer = vec![0; account_size];

// Initialize the structure for your instruction
ExtraAccountMetaList::init::<MyInstruction>(&mut buffer, &extra_metas).unwrap();

// Off-chain, you can add the additional accounts directly from the account data
// You need to provide the resolver a way to fetch account data off-chain
let client = RpcClient::new_mock("succeeds".to_string());
let program_id = Pubkey::new_unique();
let mut instruction = Instruction::new_with_bytes(program_id, &[0, 1, 2], vec![]);
ExtraAccountMetaList::add_to_instruction::<_, _, MyInstruction>(
    &mut instruction,
    |address: &Pubkey| {
        client
            .get_account(address)
            .map_ok(|acct| Some(acct.data))
        },
    &buffer,
)
.await
.unwrap();

// On-chain, you can add the additional accounts *and* account infos
let mut cpi_instruction = Instruction::new_with_bytes(program_id, &[0, 1, 2], vec![]);

// Include all of the well-known required account infos here first
let mut cpi_account_infos = vec![]; 

// Provide all "remaining_account_infos" that are *not* part of any other known interface
let remaining_account_infos = &[]; 
ExtraAccountMetaList::add_to_cpi_instruction::<MyInstruction>(
    &mut cpi_instruction,
    &mut cpi_account_infos,
    &buffer,
    &remaining_account_infos,
).unwrap();
```

For ease of use on-chain, `ExtraAccountMetaList::init` is also
provided to initialize directly from a set of given accounts.

## Motivation

The Solana account model presents unique challenges for program interfaces.
Since it's impossible to load additional accounts on-chain, if a program requires
additional accounts to properly implement an instruction, there's no clear way
for clients to fetch these accounts.

There are two main ways to fetch additional accounts, dynamically through program
simulation, or statically by fetching account data. This library implements
additional account resolution statically. You can find more information about
dynamic account resolution in the Appendix.

### Static Account Resolution

It's possible for programs to write the additional required account infos
into account data, so that on-chain and off-chain clients simply need to read
the data to figure out the additional required accounts.

Rather than exposing this data dynamically through program execution, this method
uses static account data.

For example, let's imagine there's a `Transferable` interface, along with a
`transfer` instruction. Some programs that implement `transfer` may need more
accounts than just the ones defined in the interface. How does an on-chain or
off-chain client figure out the additional required accounts?

The "static" approach requires programs to write the extra required accounts to
an account defined at a given address. This could be directly in the `mint`, or
some address derivable from the mint address.

Off-chain, a client must fetch this additional account and read its data to find
out the additional required accounts, and then include them in the instruction.

On-chain, a program must have access to "remaining account infos" containing the
special account and all other required accounts to properly create the CPI
instruction and give the correct account infos.

This approach could also be called a "state interface".

### Types of Required Accounts

This library is capable of storing two types of configurations for additional
required accounts:

- Accounts with a fixed address
- Accounts with a **dynamic program-derived address** derived from seeds that
may come from any combination of the following:
  - Hard-coded values, such as string literals or integers
  - A slice of the instruction data provided to the transfer-hook program
  - The address of another account in the total list of accounts
  - A program id from another account in the instruction

When you store configurations for a dynamic Program-Derived Address within the
additional required accounts, the PDA itself is evaluated (or resolved) at the
time of instruction invocation using the instruction itself. This
occurs in the offchain and onchain helpers mentioned below, which leverage
the SPL TLV Account Resolution library to perform this resolution
automatically.

## How it Works

This library uses `spl-type-length-value` to read and write required instruction
accounts from account data.

Interface instructions must have an 8-byte discriminator, so that the exposed
`ExtraAccountMetaList` type can use the instruction discriminator as an
`ArrayDiscriminator`, which allows that discriminator to serve as a unique TLV
discriminator for identifying entries that correspond to that particular
instruction.

This can be confusing. Typically, a type implements `SplDiscriminate`, so that
the type can be written into TLV data. In this case, `ExtraAccountMetaList` is
generic over `SplDiscriminate`, meaning that a program can write many different instances of
`ExtraAccountMetaList` into one account, using different `ArrayDiscriminator`s.

Also, it's reusing an instruction discriminator as a TLV discriminator. For example,
if the `transfer` instruction has a discriminator of `[1, 2, 3, 4, 5, 6, 7, 8]`,
then the account uses a TLV discriminator of `[1, 2, 3, 4, 5, 6, 7, 8]` to denote
where the additional account metas are stored.

This isn't required, but makes it easier for clients to find the additional
required accounts for an instruction.

## Appendix

### Dynamic Account Resolution

To expose the additional accounts required, instruction interfaces can include
supplemental instructions to return the required accounts.

For example, in the `Transferable` interface example, along with a `transfer`
instruction, also requires implementations to expose a
`get_additional_accounts_for_transfer` instruction.

In the program implementation, this instruction writes the additional accounts
into return data, making it easy for on-chain and off-chain clients to consume.

See the
[relevant sRFC](https://forum.solana.com/t/srfc-00010-additional-accounts-request-transfer-spec/122)
for more information about the dynamic approach.
