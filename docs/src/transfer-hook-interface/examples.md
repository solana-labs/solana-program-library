---
title: Examples
---

More examples can be found in the
[Transfer Hook example tests](https://github.com/solana-labs/solana-program-library/blob/master/token/transfer-hook/example/tests/functional.rs),
as well as the
[TLV Account Resolution tests](https://github.com/solana-labs/solana-program-library/blob/master/libraries/tlv-account-resolution/src/state.rs).

### Initializing Extra Account Metas On-Chain

The
[`ExtraAccountMetaList`](https://github.com/solana-labs/solana-program-library/blob/65a92e6e0a4346920582d9b3893cacafd85bb017/libraries/tlv-account-resolution/src/state.rs#L167)
struct is designed to make working with extra account
configurations as seamless as possible.

Using `ExtraAccountMetaList::init<T>(..)`, you can initialize a buffer with the
serialized `ExtraAccountMeta` configurations by simply providing a mutable
reference to the buffer and a slice of `ExtraAccountMeta`. The generic `T` is
the instruction whose discriminator the extra account configurations should be
assigned to. In our case, this will be
[`spl_transfer_hook_interface::instruction::ExecuteInstruction`](https://github.com/solana-labs/solana-program-library/blob/eb32c5e72c6d917e732bded9863db7657b23e428/token/transfer-hook/interface/src/instruction.rs#L68)
from the Transfer Hook interface.

> Note: All instructions from the SPL Transfer Hook interface implement the
> trait
> [`SplDiscriminate`](https://github.com/solana-labs/solana-program-library/blob/65a92e6e0a4346920582d9b3893cacafd85bb017/libraries/discriminator/src/discriminator.rs#L9),
> which provides a constant 8-byte discriminator that
> can be used to create a TLV data entry.

```rust
pub fn process_initialize_extra_account_meta_list(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    extra_account_metas: &[ExtraAccountMeta],
) -> ProgramResult {
  let account_info_iter = &mut accounts.iter();

  let validation_info = next_account_info(account_info_iter)?;
  let mint_info = next_account_info(account_info_iter)?;
  let authority_info = next_account_info(account_info_iter)?;
  let _system_program_info = next_account_info(account_info_iter)?;

  // Check validation account
  let (expected_validation_address, bump_seed) =
      get_extra_account_metas_address_and_bump_seed(mint_info.key, program_id);
  if expected_validation_address != *validation_info.key {
      return Err(ProgramError::InvalidSeeds);
  }

  // Create the account
  let bump_seed = [bump_seed];
  let signer_seeds = collect_extra_account_metas_signer_seeds(mint_info.key, &bump_seed);
  let length = extra_account_metas.len();
  let account_size = ExtraAccountMetaList::size_of(length)?;
  invoke_signed(
      &system_instruction::allocate(validation_info.key, account_size as u64),
      &[validation_info.clone()],
      &[&signer_seeds],
  )?;
  invoke_signed(
      &system_instruction::assign(validation_info.key, program_id),
      &[validation_info.clone()],
      &[&signer_seeds],
  )?;

  // Write the data
  let mut data = validation_info.try_borrow_mut_data()?;
  ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, extra_account_metas)?;

  Ok(())
}
```

After calling `ExtraAccountMetaList::init::<ExecuteInstruction>(..)` on the
mutable account data, the account now stores all of the serialized extra account
configurations for an `Execute` instruction!

### Resolving Extra Account Metas Off-Chain

When building a transaction with an instruction, either for your transfer hook
program directly or for a program that will CPI to your transfer hook program,
you must include all required accounts - including the extra accounts.

Below is an example of the logic contained in the Transfer Hook interface's
[offchain helper](https://github.com/solana-labs/solana-program-library/blob/65a92e6e0a4346920582d9b3893cacafd85bb017/token/transfer-hook/interface/src/offchain.rs#L50).

```rust
// You'll need to provide an "account data function", which is a function that
// can, given a `Pubkey`, return account data within an `AccountDataResult`.
// This is most likely based off of an RPC call like `getAccountInfo`.

// Load the validation state data
let validate_state_pubkey = get_extra_account_metas_address(mint_pubkey, program_id);
let validate_state_data = fetch_account_data_fn(validate_state_pubkey)
    .await?
    .ok_or(ProgramError::InvalidAccountData)?;


// First create an `ExecuteInstruction`
let mut execute_instruction = execute(
    program_id,
    source_pubkey,
    mint_pubkey,
    destination_pubkey,
    authority_pubkey,
    &validate_state_pubkey,
    amount,
);

// Resolve all additional required accounts for `ExecuteInstruction`
ExtraAccountMetaList::add_to_instruction::<ExecuteInstruction, _, _>(
    &mut execute_instruction,
    fetch_account_data_fn,
    &validate_state_data,
)
.await?;

// Add only the extra accounts resolved from the validation state
instruction
    .accounts
    .extend_from_slice(&execute_instruction.accounts[5..]);

// Add the program id and validation state account
instruction
    .accounts
    .push(AccountMeta::new_readonly(*program_id, false));
instruction
    .accounts
    .push(AccountMeta::new_readonly(validate_state_pubkey, false));
```

As you can see from the example, an important concept to remember is which
instruction these extra accounts are for. Even though you might be building an
instruction for some other program, which may not need them, if that program is
going to CPI to your transfer hook program, it needs to have the proper
accounts.

Additionally, in order to perform a successful dynamic account resolution, the
proper instruction needs to be provided to align with the instruction that was
configured in the validation account - in this case the Transfer Hook
interface's `ExecuteInstruction`. This is why we first create an
`ExecuteInstruction`, then resolve the extra accounts for that instruction, and
finally add those accounts to our current instruction.

### Resolving Extra Account Metas On-Chain for CPI

During the execution of a program that seeks to CPI to your transfer hook
program, even though the additional required accounts were provided by the
offchain account resolution, the executing program has to know how to build a
CPI instruction with the proper accounts as well!

Below is an example of the logic contained in the Transfer Hook interface's
[onchain helper](https://github.com/solana-labs/solana-program-library/blob/65a92e6e0a4346920582d9b3893cacafd85bb017/token/transfer-hook/interface/src/onchain.rs#L67).

```rust
// Find the validation account from the list of `AccountInfo`s and load its
// data
let validate_state_pubkey = get_extra_account_metas_address(mint_info.key, program_id);
let validate_state_info = account_infos
    .iter()
    .find(|&x| *x.key == validate_state_pubkey)
    .ok_or(TransferHookError::IncorrectAccount)?;

// Find the transfer hook program ID
let program_info = account_infos
    .iter()
    .find(|&x| x.key == program_id)
    .ok_or(TransferHookError::IncorrectAccount)?;

// First create an `ExecuteInstruction`
let mut execute_instruction = instruction::execute(
    program_id,
    source_info.key,
    mint_info.key,
    destination_info.key,
    authority_info.key,
    &validate_state_pubkey,
    amount,
);
let mut execute_account_infos = vec![
    source_info,
    mint_info,
    destination_info,
    authority_info,
    validate_state_info.clone(),
];

// Resolve all additional required accounts for `ExecuteInstruction`
ExtraAccountMetaList::add_to_cpi_instruction::<instruction::ExecuteInstruction>(
    &mut execute_instruction,
    &mut execute_account_infos,
    &validate_state_info.try_borrow_data()?,
    account_infos,
)?;

// Add only the extra accounts resolved from the validation state
cpi_instruction
    .accounts
    .extend_from_slice(&execute_instruction.accounts[5..]);
cpi_account_infos.extend_from_slice(&execute_account_infos[5..]);

// Add the program id and validation state account
cpi_instruction
    .accounts
    .push(AccountMeta::new_readonly(*program_id, false));
cpi_instruction
    .accounts
    .push(AccountMeta::new_readonly(validate_state_pubkey, false));
cpi_account_infos.push(program_info.clone());
cpi_account_infos.push(validate_state_info.clone());
```

Although this example may appear more verbose than its offchain counterpart,
it's actually doing the exact same steps, just with an instruction _and_ a list
of account infos, since CPI requires both.

The key difference between `ExtraAccountMetaList::add_to_instruction(..)` and
`ExtraAccountMetaList::add_to_cpi_instruction(..)` is that the latter method
will find the corresponding `AccountInfo` in the list and add it to
`cpi_account_infos` at the same time as it adds the resolved `AccountMeta` to
the instruction, ensuring all resolved account keys are present in the
`AccountInfo` list.
