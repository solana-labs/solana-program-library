## Transfer-Hook Interface

### Example program

Here is an example program that only implements the required "execute" instruction,
assuming that the proper account data is already written to the appropriate 
program-derived address defined by the interface.

```rust
use {
    solana_program::{entrypoint::ProgramResult, program_error::ProgramError},
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction},
    spl_type_length_value::state::TlvStateBorrowed,
};
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TransferHookInstruction::unpack(input)?;
    let _amount = match instruction {
        TransferHookInstruction::Execute { amount } => amount,
        _ => return Err(ProgramError::InvalidInstructionData),
    };
    let account_info_iter = &mut accounts.iter();

    // Pull out the accounts in order, none are validated in this test program
    let _source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let _authority_info = next_account_info(account_info_iter)?;
    let extra_account_metas_info = next_account_info(account_info_iter)?;

    // Only check that the correct pda and account are provided
    let expected_validation_address = get_extra_account_metas_address(mint_info.key, program_id);
    if expected_validation_address != *extra_account_metas_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Get the extra account metas from the account data
    let data = extra_account_metas_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&data).unwrap();
    let extra_account_metas = 
        ExtraAccountMetas::unpack_with_tlv_state::<ExecuteInstruction>(&state)?;

    // If incorrect number of accounts is provided, error
    let extra_account_infos = account_info_iter.as_slice();
    let account_metas = extra_account_metas.data();
    if extra_account_infos.len() != account_metas.len() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Let's require that they're provided in the correct order
    for (i, account_info) in extra_account_infos.iter().enumerate() {
        if &account_metas[i] != account_info {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    Ok(())
}
```

### Motivation

Token creators may need more control over transfers of their token. The most
prominent use case revolves around NFT royalties. Whenever a token is moved,
the creator should be entitled to royalties, but due to the design of the current
token program, it's impossible to stop a transfer at the protocol level.

Current solutions typically resort to perpetually freezing tokens, which requires
a whole proxy layer to interact with the token. Wallets and marketplaces need
to be aware of the proxy layer in order to properly use the token.

Worse still, different royalty systems have different proxy layers for using
their token. All in all, these systems harm composability and make development
harder.

### Solution

To give more flexibility to token creators and improve the situation for everyone,
`spl-transfer-hook-interface` introduces the concept of an interface integrated
with `spl-token-2022`. A token creator must develop and deploy a program that
implements the interface and then configure their token mint to use their program.

During transfer, token-2022 calls into the program with the accounts specified
at a well-defined program-derived address for that mint and program id. This
call happens after all other transfer logic, so the accounts reflect the *end*
state of the transfer.

A developer must implement the `Execute` instruction, and the
`InitializeExtraAccountMetas` instruction to write the required additional account
pubkeys into the program-derived address defined by the mint and program id.

Side note: it's technically not required to implement `InitializeExtraAccountMetas`
at that instruction descriminator. Your program may implement multiple interfaces,
so any other instruction in your program can create the account at the program-derived
address!

This library provides offchain and onchain helpers for resolving the additional
accounts required. See
[invoke.rs](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook-interface/src/invoke.rs)
for usage on-chain, and
[offchain.rs](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook-interface/src/offchain.rs)
for fetching the additional required account metas.
