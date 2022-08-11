//! Upgrades the program from the buffer

use {
    solana_farm_sdk::program::account,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        bpf_loader_upgradeable,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        loader_upgradeable_instruction::UpgradeableLoaderInstruction,
        msg,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
        sysvar,
    },
};

pub fn upgrade_program(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    multisig_bump: u8,
) -> ProgramResult {
    msg!("Processing MainInstruction::UpgradeProgram");

    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let target_program = next_account_info(accounts_iter)?;
    let program_buffer = next_account_info(accounts_iter)?;
    let source_buffer = next_account_info(accounts_iter)?;
    let rent_program = next_account_info(accounts_iter)?;
    let clock_program = next_account_info(accounts_iter)?;

    let current_authority = if account::is_empty(multisig_account)? {
        signer_account.key
    } else {
        multisig_account.key
    };

    let instruction = Instruction::new_with_bincode(
        bpf_loader_upgradeable::id(),
        &UpgradeableLoaderInstruction::Upgrade,
        vec![
            AccountMeta::new(*program_buffer.key, false),
            AccountMeta::new(*target_program.key, false),
            AccountMeta::new(*source_buffer.key, false),
            AccountMeta::new(*signer_account.key, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(*current_authority, true),
        ],
    );

    if account::is_empty(multisig_account)? {
        invoke(
            &instruction,
            &[
                program_buffer.clone(),
                target_program.clone(),
                source_buffer.clone(),
                signer_account.clone(),
                rent_program.clone(),
                clock_program.clone(),
                signer_account.clone(),
            ],
        )?;
    } else {
        invoke_signed(
            &instruction,
            &[
                program_buffer.clone(),
                target_program.clone(),
                source_buffer.clone(),
                signer_account.clone(),
                rent_program.clone(),
                clock_program.clone(),
                multisig_account.clone(),
            ],
            &[&[b"multisig", target_program.key.as_ref(), &[multisig_bump]]],
        )?;
    }

    msg!("UpgradeProgram complete");

    Ok(())
}
