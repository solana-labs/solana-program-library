//! Themis program
use crate::instruction::ThemisInstruction;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    pubkey::Pubkey,
};

fn process_calculate_aggregate(
    _user_info: &AccountInfo,
    _policies_info: &AccountInfo,
) -> Result<(), ProgramError> {
    //let user = User::unpack(&user_account.data.borrow_mut())?;
    //let policies = Policies::unpack(&policies_account.data.borrow_mut())?;
    Ok(())
}

/// Process the given transaction instruction
pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    account_infos: &'a [AccountInfo<'a>],
    input: &[u8],
) -> Result<(), ProgramError> {
    let account_infos_iter = &mut account_infos.iter();
    let instruction = ThemisInstruction::unpack(input)?;

    match instruction {
        ThemisInstruction::InitializeUserAccount => {
            let _user_info = next_account_info(account_infos_iter)?;
            //process_initialize_user_account(&user_info)
            Ok(())
        }
        ThemisInstruction::InitializePoliciesAccount { policies: _ } => {
            let _policies_info = next_account_info(account_infos_iter)?;
            //process_initialize_policies_account(&policies_info)
            Ok(())
        }
        ThemisInstruction::CalculateAggregate {
            encrypted_interactions: _,
            public_key: _,
        } => {
            let user_info = next_account_info(account_infos_iter)?;
            let policies_info = next_account_info(account_infos_iter)?;
            process_calculate_aggregate(&user_info, &policies_info)
        }
        ThemisInstruction::SubmitProofDecryption {
            plaintext: _,
            announcement_g: _,
            announcement_ctx: _,
            response: _,
        } => {
            let _user_info = next_account_info(account_infos_iter)?;
            Ok(())
        }
        ThemisInstruction::RequestPayment {
            encrypted_aggregate: _,
            decrypted_aggregate: _,
            proof_correct_decryption: _,
        } => {
            let _user_info = next_account_info(account_infos_iter)?;
            Ok(())
        }
    }
}
