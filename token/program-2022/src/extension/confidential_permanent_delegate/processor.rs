#[cfg(feature = "zk-ops")]
use {
    super::instruction::ApproveAccountData,
    crate::extension::confidential_transfer::verify_proof::verify_configure_account_proof,
};
use {
    super::{
        encrypted_keys_pda_address, encrypted_keys_pda_address_bump, encrypted_keys_pda_seed,
        instruction::{
            ConfidentialPermanentDelegateInstruction, PostEncryptedKeysInstructionData,
            PrivateKeyType,
        },
        ConfidentialPermanentDelegate, EncyptionPublicKey, MAX_MODULUS_LENGTH,
    },
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_permanent_delegate::instruction::{
                ConfigureRSAInstructionData, EncryptedPrivateKeyData, InitializeMintData,
                UpdateMintData,
            },
            confidential_transfer::ConfidentialTransferAccount,
            BaseStateWithExtensions, BaseStateWithExtensionsMut, PodStateWithExtensions,
            PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::{PodAccount, PodMint},
        processor::Processor,
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
    spl_pod::bytemuck::pod_get_packed_len,
};

/// Processes an [InitializeMint] instruction.
fn process_initialize_mint(accounts: &[AccountInfo], permanent_delegate: Pubkey) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;

    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let whitelist_transfer_mint = mint.init_extension::<ConfidentialPermanentDelegate>(true)?;

    whitelist_transfer_mint.permanent_delegate = permanent_delegate;

    Ok(())
}

/// Processes an [UpdateMint] instruction.
fn process_update_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_delegate: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let permanent_delegate_info = next_account_info(account_info_iter)?;
    let delegate_info_data_len = permanent_delegate_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let confidential_pd_mint = mint.get_extension_mut::<ConfidentialPermanentDelegate>()?;

    Processor::validate_owner(
        program_id,
        &confidential_pd_mint.permanent_delegate,
        permanent_delegate_info,
        delegate_info_data_len,
        account_info_iter.as_slice(),
    )?;

    confidential_pd_mint.permanent_delegate = new_delegate;

    Ok(())
}

/// Processes a [ConfigureRSA] instruction.
#[cfg(feature = "zk-ops")]
fn process_configure_rsa(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    rsa_pubkey: EncyptionPublicKey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let permanent_delegate_info = next_account_info(account_info_iter)?;
    let delegate_info_data_len = permanent_delegate_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let confidential_pd_mint = mint.get_extension_mut::<ConfidentialPermanentDelegate>()?;

    Processor::validate_owner(
        program_id,
        &confidential_pd_mint.permanent_delegate,
        permanent_delegate_info,
        delegate_info_data_len,
        account_info_iter.as_slice(),
    )?;

    confidential_pd_mint.encryption_pubkey = rsa_pubkey;
    confidential_pd_mint.delegate_initialized = true.into();

    Ok(())
}

/// Processes a [PostEncryptedPrivateKeys] instruction.
#[cfg(feature = "zk-ops")]
fn process_post_encrypted_keys(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &PostEncryptedKeysInstructionData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let enc_key_pda_info = next_account_info(account_info_iter)?;
    let ata_authority_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_payer_info = next_account_info(account_info_iter)?;

    if !ata_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mint_data = &mint_info.data.borrow();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;

    if mint
        .get_extension::<ConfidentialPermanentDelegate>()
        .is_err()
    {
        return Err(TokenError::ExtensionNotFound.into());
    }

    let pda_seed = encrypted_keys_pda_seed(mint_info.key, token_account_info.key);
    let (pda_address, pda_bump) = encrypted_keys_pda_address_bump(pda_seed, program_id);
    if &pda_address != enc_key_pda_info.key {
        msg!("calculated encrypted key pda and supplied account info do not match");
        return Err(ProgramError::InvalidInstructionData);
    }

    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    let confidential_transfer_state =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    if confidential_transfer_state.approved.into() {
        msg!("cannot alter posted keys on already approved token accounts");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let num_bytes = pod_get_packed_len::<EncryptedPrivateKeyData>();
    let pda_rent = Rent::get()?.minimum_balance(num_bytes);

    let key_type: PrivateKeyType = data
        .key_type
        .try_into()
        .or(Err(ProgramError::InvalidInstructionData))?;

    if enc_key_pda_info.lamports() != pda_rent {
        invoke_signed(
            &system_instruction::create_account(
                rent_payer_info.key,
                &pda_address,
                pda_rent,
                num_bytes as u64,
                program_id,
            ),
            &[
                rent_payer_info.clone(),
                enc_key_pda_info.clone(),
                system_program_info.clone(),
            ],
            &[&[&pda_seed, &[pda_bump]]], // signature
        )?;
    }

    let mut pda_data = enc_key_pda_info.data.borrow_mut();
    match key_type {
        PrivateKeyType::ElGamalKeypair => {
            pda_data[..MAX_MODULUS_LENGTH].copy_from_slice(&data.data)
        }
        PrivateKeyType::AeKey => pda_data[MAX_MODULUS_LENGTH..].copy_from_slice(&data.data),
    }

    Ok(())
}

/// Processes a [ApproveAccount] instruction.
#[cfg(feature = "zk-ops")]
fn process_approve_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &ApproveAccountData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let enc_key_pda_info = next_account_info(account_info_iter)?;
    let permanent_delegate_info = next_account_info(account_info_iter)?;

    // zero-knowledge proof certifies that the supplied ElGamal public key is valid
    let proof_context =
        verify_configure_account_proof(account_info_iter, data.proof_instruction_offset as i64)?;

    if !permanent_delegate_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mint_data = &mint_info.data.borrow();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;

    if mint
        .get_extension::<ConfidentialPermanentDelegate>()
        .is_err()
    {
        return Err(TokenError::ExtensionNotFound.into());
    }

    let pda_address = encrypted_keys_pda_address(mint_info.key, token_account_info.key, program_id);
    if &pda_address != enc_key_pda_info.key {
        msg!("calculated encrypted key pda and supplied account info do not match");
        return Err(ProgramError::InvalidInstructionData);
    }

    if enc_key_pda_info.lamports()
        < Rent::get()?.minimum_balance(pod_get_packed_len::<EncryptedPrivateKeyData>())
    {
        return Err(ProgramError::UninitializedAccount);
    }

    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    let confidential_transfer_state =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    if confidential_transfer_state.elgamal_pubkey != proof_context.pubkey {
        msg!("elgamal pubkeys from proof and token account don't match");
        return Err(ProgramError::InvalidInstructionData);
    }

    confidential_transfer_state.approved = true.into();

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        ConfidentialPermanentDelegateInstruction::InitializeMint => {
            msg!("ConfidentialPermanentDelegateInstruction::InitializeMint");
            let data = decode_instruction_data::<InitializeMintData>(input)?;
            process_initialize_mint(accounts, data.permanent_delegate)
        }
        ConfidentialPermanentDelegateInstruction::UpdateMint => {
            msg!("ConfidentialPermanentDelegateInstruction::UpdateMint");
            let data = decode_instruction_data::<UpdateMintData>(input)?;
            process_update_mint(program_id, accounts, data.new_permanent_delegate)
        }
        ConfidentialPermanentDelegateInstruction::ConfigureRSA => {
            msg!("ConfidentialPermanentDelegateInstruction::ConfigureRSA");
            let data = decode_instruction_data::<ConfigureRSAInstructionData>(input)?;
            process_configure_rsa(program_id, accounts, data.rsa_pubkey)
        }
        ConfidentialPermanentDelegateInstruction::PostEncryptedPrivateKey => {
            msg!("ConfidentialPermanentDelegateInstruction::PostEncryptedPrivateKeys");
            let data = decode_instruction_data::<PostEncryptedKeysInstructionData>(input)?;
            process_post_encrypted_keys(program_id, accounts, data)
        }
        ConfidentialPermanentDelegateInstruction::ApproveAccount => {
            msg!("ConfidentialPermanentDelegateInstruction::ApproveAccount");
            let data = decode_instruction_data::<ApproveAccountData>(input)?;
            process_approve_account(program_id, accounts, data)
        }
    }
}
