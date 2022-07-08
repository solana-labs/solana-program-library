//! Sets single upgrade authority for the program removing multisig if present

use {
    solana_farm_sdk::{id::zero, program::account},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        bpf_loader_upgradeable,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        loader_upgradeable_instruction::UpgradeableLoaderInstruction,
        msg,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
    },
};

pub fn set_program_single_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    multisig_bump: u8,
) -> ProgramResult {
    msg!("Processing MainInstruction::SetProgramSingleAuthority");

    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let target_program = next_account_info(accounts_iter)?;
    let program_buffer = next_account_info(accounts_iter)?;
    let new_authority = next_account_info(accounts_iter)?;

    let current_authority = if account::is_empty(multisig_account)? {
        signer_account.key
    } else {
        multisig_account.key
    };

    let mut metas = vec![
        AccountMeta::new(*program_buffer.key, false),
        AccountMeta::new_readonly(*current_authority, true),
    ];
    if new_authority.key != &zero::id() {
        metas.push(AccountMeta::new_readonly(*new_authority.key, false));
    }
    let instruction = Instruction::new_with_bincode(
        bpf_loader_upgradeable::id(),
        &UpgradeableLoaderInstruction::SetAuthority,
        metas,
    );

    if account::is_empty(multisig_account)? {
        invoke(
            &instruction,
            &[
                program_buffer.clone(),
                signer_account.clone(),
                new_authority.clone(),
            ],
        )?;
    } else {
        invoke_signed(
            &instruction,
            &[
                program_buffer.clone(),
                multisig_account.clone(),
                new_authority.clone(),
            ],
            &[&[b"multisig", target_program.key.as_ref(), &[multisig_bump]]],
        )?;
        account::close_system_account(signer_account, multisig_account, program_id)?;
    }

    msg!("SetProgramSingleAuthority complete");

    Ok(())
}
