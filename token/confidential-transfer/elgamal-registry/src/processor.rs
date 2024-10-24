use {
    crate::{
        get_elgamal_registry_address_and_bump_seed,
        instruction::RegistryInstruction,
        state::{ElGamalRegistry, ELGAMAL_REGISTRY_ACCOUNT_LEN},
        REGISTRY_ADDRESS_SEED,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    solana_zk_sdk::zk_elgamal_proof_program::proof_data::pubkey_validity::{
        PubkeyValidityProofContext, PubkeyValidityProofData,
    },
    spl_pod::bytemuck::pod_from_bytes_mut,
    spl_token_confidential_transfer_proof_extraction::instruction::verify_and_extract_context,
};

/// Processes `CreateRegistry` instruction
pub fn process_create_registry_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let elgamal_registry_account_info = next_account_info(account_info_iter)?;
    let wallet_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    if !wallet_account_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // zero-knowledge proof certifies that the supplied ElGamal public key is valid
    let proof_context = verify_and_extract_context::<
        PubkeyValidityProofData,
        PubkeyValidityProofContext,
    >(account_info_iter, proof_instruction_offset, None)?;

    let (elgamal_registry_account_address, bump_seed) =
        get_elgamal_registry_address_and_bump_seed(wallet_account_info.key, program_id);
    if elgamal_registry_account_address != *elgamal_registry_account_info.key {
        msg!("Error: ElGamal registry account address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    let elgamal_registry_account_seeds: &[&[_]] = &[
        REGISTRY_ADDRESS_SEED,
        wallet_account_info.key.as_ref(),
        &[bump_seed],
    ];
    let rent = Rent::get()?;

    create_pda_account(
        &rent,
        ELGAMAL_REGISTRY_ACCOUNT_LEN,
        program_id,
        system_program_info,
        elgamal_registry_account_info,
        elgamal_registry_account_seeds,
    )?;

    let elgamal_registry_account_data = &mut elgamal_registry_account_info.data.borrow_mut();
    let elgamal_registry_account =
        pod_from_bytes_mut::<ElGamalRegistry>(elgamal_registry_account_data)?;
    elgamal_registry_account.owner = *wallet_account_info.key;
    elgamal_registry_account.elgamal_pubkey = proof_context.pubkey;

    Ok(())
}

/// Processes `UpdateRegistry` instruction
pub fn process_update_registry_account(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let elgamal_registry_account_info = next_account_info(account_info_iter)?;
    let elgamal_registry_account_data = &mut elgamal_registry_account_info.data.borrow_mut();
    let elgamal_registry_account =
        pod_from_bytes_mut::<ElGamalRegistry>(elgamal_registry_account_data)?;

    // zero-knowledge proof certifies that the supplied ElGamal public key is valid
    let proof_context = verify_and_extract_context::<
        PubkeyValidityProofData,
        PubkeyValidityProofContext,
    >(account_info_iter, proof_instruction_offset, None)?;

    let owner_info = next_account_info(account_info_iter)?;
    validate_owner(owner_info, &elgamal_registry_account.owner)?;

    elgamal_registry_account.elgamal_pubkey = proof_context.pubkey;
    Ok(())
}

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = RegistryInstruction::unpack(input)?;
    match instruction {
        RegistryInstruction::CreateRegistry {
            proof_instruction_offset,
        } => {
            msg!("ElGamalRegistryInstruction::CreateRegistry");
            process_create_registry_account(program_id, accounts, proof_instruction_offset as i64)
        }
        RegistryInstruction::UpdateRegistry {
            proof_instruction_offset,
        } => {
            msg!("ElGamalRegistryInstruction::UpdateRegistry");
            process_update_registry_account(program_id, accounts, proof_instruction_offset as i64)
        }
    }
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

/// Allocate ElGamal registry account using Program Derived Address for the
/// given seeds
pub fn create_pda_account<'a>(
    rent: &Rent,
    space: usize,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    new_pda_account: &AccountInfo<'a>,
    new_pda_signer_seeds: &[&[u8]],
) -> ProgramResult {
    let required_lamports = rent
        .minimum_balance(space)
        .saturating_sub(new_pda_account.lamports());

    if required_lamports > 0 {
        return Err(ProgramError::AccountNotRentExempt);
    }

    invoke_signed(
        &system_instruction::allocate(new_pda_account.key, space as u64),
        &[new_pda_account.clone(), system_program.clone()],
        &[new_pda_signer_seeds],
    )?;

    invoke_signed(
        &system_instruction::assign(new_pda_account.key, owner),
        &[new_pda_account.clone(), system_program.clone()],
        &[new_pda_signer_seeds],
    )
}
