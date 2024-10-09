use {
    crate::{instruction::RegistryInstruction, state::ElGamalRegistry},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    solana_zk_sdk::zk_elgamal_proof_program::proof_data::pubkey_validity::{
        PubkeyValidityProofContext, PubkeyValidityProofData,
    },
    spl_pod::bytemuck::pod_from_bytes_mut,
    spl_token_confidential_transfer_proof_extraction::verify_and_extract_context,
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = RegistryInstruction::unpack(input)?;
    let account_info_iter = &mut accounts.iter();
    let registry_account_info = next_account_info(account_info_iter)?;
    let registry_account_data = &mut registry_account_info.data.borrow_mut();
    let registry_account = pod_from_bytes_mut::<ElGamalRegistry>(registry_account_data)?;

    let proof_instruction_offset = match instruction {
        RegistryInstruction::CreateRegistry {
            owner,
            proof_instruction_offset,
        } => {
            // set the owner; ElGamal pubkey is set after the zkp verification below
            registry_account.owner = owner;
            proof_instruction_offset
        }
        RegistryInstruction::UpdateRegistry {
            proof_instruction_offset,
        } => {
            // check the owner; ElGamal pubkey is set after the zkp verification below
            let owner_info = next_account_info(account_info_iter)?;
            validate_owner(owner_info, &registry_account.owner)?;
            proof_instruction_offset
        }
    };
    // zero-knowledge proof certifies that the supplied ElGamal public key is valid
    let proof_context = verify_and_extract_context::<
        PubkeyValidityProofData,
        PubkeyValidityProofContext,
    >(account_info_iter, proof_instruction_offset as i64, None)?;
    registry_account.elgamal_pubkey = proof_context.pubkey;

    Ok(())
}

fn validate_owner(owner_info: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
    if expected_owner != owner_info.key {
        return Err(ProgramError::InvalidAccountOwner);
    }
    if !owner_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}
