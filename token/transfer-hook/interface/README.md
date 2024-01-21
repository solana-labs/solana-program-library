## Transfer-Hook Interface

### Example program

Here is an example program that only implements the required "execute" instruction,
assuming that the proper account data is already written to the appropriate 
program-derived address defined by the interface.

```rust
use {
    solana_program::{entrypoint::ProgramResult, program_error::ProgramError},
    spl_tlv_account_resolution::state::ExtraAccountMetaList,
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

    // Load the extra required accounts from the validation account
    let data = extra_account_metas_info.try_borrow_data()?;

    // Check the provided accounts against the validation data
    ExtraAccountMetaList::check_account_infos::<ExecuteInstruction>(
        accounts,
        &TransferHookInstruction::Execute { amount }.pack(),
        program_id,
        &data,
    )?;

    Ok(())
}
```

### Motivation

Token creators may need more control over how their token is transferred. The
most prominent use case revolves around NFT royalties. Whenever a token is moved,
the creator should be entitled to royalties, but due to the design of the current
token program, it's impossible to stop a transfer at the protocol level.

Current solutions typically resort to perpetually freezing tokens, which requires
a whole proxy layer to interact with the token. Wallets and marketplaces need
to be aware of the proxy layer in order to properly use the token.

Worse still, different royalty systems have different proxy layers for using
their token. All in all, these systems harm composability and make development
harder.

### Solution

To improve the situation, Token-2022 introduces the concept of the transfer-hook
interface and extension. A token creator must develop and deploy a program that
implements the interface and then configure their token mint to use their program.

During transfer, Token-2022 calls into the program with the accounts specified
at a well-defined program-derived address for that mint and program id. This
call happens after all other transfer logic, so the accounts reflect the *end*
state of the transfer.

### How to Use

Developers must implement the `Execute` instruction, and optionally the
`InitializeExtraAccountMetaList` instruction to write the required additional account
pubkeys into the program-derived address defined by the mint and program id.

Note: it's technically not required to implement `InitializeExtraAccountMetaList`
at that instruction descriminator. Your program may implement multiple interfaces,
so any other instruction in your program can create the account at the program-derived
address!

When your program stores configurations for extra required accounts in the
well-defined program-derived address, it's possible to send an instruction -
such as `Execute` (transfer) - to your program with only accounts required
for the interface instruction, and all extra required accounts are
automatically resolved!

### Account Resolution

Implementers of the transfer-hook interface are encouraged to make use of the
[spl-tlv-account-resolution](https://github.com/solana-labs/solana-program-library/tree/master/libraries/tlv-account-resolution/README.md)
library to manage the additional required accounts for their transfer hook
program.

TLV Account Resolution is capable of powering on-chain account resolution
when an instruction that requires extra accounts is invoked.
Read more about how account resolution works in the repository's
[README file](https://github.com/solana-labs/solana-program-library/tree/master/libraries/tlv-account-resolution/README.md).

### An Example

You have created a DAO to govern a community. Your DAO's authority is a
multisig account, and you want to ensure that any transfer of your token is
approved by the DAO. You also want to make sure that someone who intends to
transfer your token has the proper permissions to do so.

Let's assume the DAO multisig has some **fixed address**. And let's assume that
in order to have the `can_transfer` permission, a user must have this
**dynamic program-derived address** associated with their wallet via the
following seeds: `"can_transfer" + <wallet_address>`.

Using the transfer-hook interface, you can store these configurations in the
well-defined program-derived address for your mint and program id.

When a user attempts to transfer your token, they might provide to Token-2022:

```rust
[source, mint, destination, owner/delegate]
```

Token-2022 will then call into your program,
**resolving the extra required accounts automatically** from your stored
configurations, to result in the following accounts being provided to your
program:

```rust
[source, mint, destination, owner/delegate, dao_authority, can_transfer_pda]
```

### Utilities

The `spl-transfer-hook-interface` library provides offchain and onchain helpers
for resolving the additional accounts required. See
[onchain.rs](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook/interface/src/onchain.rs)
for usage on-chain, and
[offchain.rs](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook/interface/src/offchain.rs)
for fetching the additional required account metas with any async off-chain client
like `BanksClient` or `RpcClient`.
