//! Program state processor

use {
    crate::{error::SlashingError, instruction::SlashingInstruction, state::ProofData},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_error::ProgramError,
        program_pack::IsInitialized,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction, system_program,
    },
    spl_pod::bytemuck::{pod_from_bytes, pod_from_bytes_mut},
};

fn check_authority(authority_info: &AccountInfo, expected_authority: &Pubkey) -> ProgramResult {
    if expected_authority != authority_info.key {
        msg!("Incorrect proof authority provided");
        return Err(SlashingError::IncorrectAuthority.into());
    }
    if !authority_info.is_signer {
        msg!("Proof authority signature missing");
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = SlashingInstruction::unpack(input)?;
    let account_info_iter = &mut accounts.iter();

    match instruction {
        SlashingInstruction::InitializeProofAccount { proof_type } => {
            msg!(
                "SlashingInstruction::InitializeProofAccount {:?}",
                proof_type
            );

            let proof_data_info = next_account_info(account_info_iter)?;
            let payer_info = next_account_info(account_info_iter)?;
            let authority_info = next_account_info(account_info_iter)?;
            let system_program_info = next_account_info(account_info_iter)?;
            let account_length = proof_type.proof_account_length()?;

            if *system_program_info.key != system_program::ID {
                msg!("Missing system program account");
                return Err(ProgramError::InvalidAccountData);
            }

            msg!("Creating proof account with size {}", account_length);
            invoke(
                &system_instruction::create_account(
                    payer_info.key,
                    proof_data_info.key,
                    1.max(Rent::default().minimum_balance(account_length)),
                    account_length as u64,
                    program_id,
                ),
                &[payer_info.clone(), proof_data_info.clone()],
            )?;

            let raw_data = &mut proof_data_info.data.borrow_mut();
            let account_data =
                pod_from_bytes_mut::<ProofData>(&mut raw_data[..ProofData::WRITABLE_START_INDEX])?;

            account_data.proof_type = u8::from(proof_type);
            account_data.authority = *authority_info.key;
            account_data.version = ProofData::CURRENT_VERSION;
            Ok(())
        }

        SlashingInstruction::Write { offset, data } => {
            msg!("SlashingInstruction::Write");
            let proof_data_info = next_account_info(account_info_iter)?;
            let authority_info = next_account_info(account_info_iter)?;
            {
                let raw_data = &proof_data_info.data.borrow();
                if raw_data.len() < ProofData::WRITABLE_START_INDEX {
                    return Err(ProgramError::InvalidAccountData);
                }
                let proof_data =
                    pod_from_bytes::<ProofData>(&raw_data[..ProofData::WRITABLE_START_INDEX])?;
                if !proof_data.is_initialized() {
                    msg!("Proof account not initialized");
                    return Err(ProgramError::UninitializedAccount);
                }
                check_authority(authority_info, &proof_data.authority)?;
            }
            msg!(
                "Writing {} bytes at {} into {}",
                data.len(),
                offset,
                proof_data_info.key
            );
            let start = ProofData::WRITABLE_START_INDEX.saturating_add(offset as usize);
            let end = start.saturating_add(data.len());
            if end > proof_data_info.data.borrow().len() {
                Err(ProgramError::AccountDataTooSmall)
            } else {
                proof_data_info.data.borrow_mut()[start..end].copy_from_slice(data);
                Ok(())
            }
        }

        SlashingInstruction::CloseAccount => {
            msg!("SlashingInstruction::CloseAccount");
            let proof_data_info = next_account_info(account_info_iter)?;
            let authority_info = next_account_info(account_info_iter)?;
            let destination_info = next_account_info(account_info_iter)?;
            let raw_data = &mut proof_data_info.data.borrow_mut();
            if raw_data.len() < ProofData::WRITABLE_START_INDEX {
                return Err(ProgramError::InvalidAccountData);
            }
            let account_data =
                pod_from_bytes_mut::<ProofData>(&mut raw_data[..ProofData::WRITABLE_START_INDEX])?;
            if !account_data.is_initialized() {
                msg!("Proof Account not initialized");
                return Err(ProgramError::UninitializedAccount);
            }
            check_authority(authority_info, &account_data.authority)?;
            let destination_starting_lamports = destination_info.lamports();
            let data_lamports = proof_data_info.lamports();
            **proof_data_info.lamports.borrow_mut() = 0;
            **destination_info.lamports.borrow_mut() = destination_starting_lamports
                .checked_add(data_lamports)
                .ok_or(SlashingError::Overflow)?;
            Ok(())
        }
    }
}
