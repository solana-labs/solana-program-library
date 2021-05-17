use {
    crate::{
        error::MetadataError,
        instruction::MetadataInstruction,
        state::{
            AccountType, MetadataEntry, SerializationMethod, VersionedIdl, CLASS_PREFIX,
            MAX_NAME_LENGTH, MAX_URL_LENGTH, MAX_VALUE_LENGTH, METADATA_ENTRY_SIZE,
            VERSIONED_IDL_SIZE,
        },
        utils::{
            assert_program_authority_has_authority_over_program,
            assert_program_matches_program_data_address, create_name_service_account,
            delete_name_service_account, update_name_service_account,
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MetadataInstruction::try_from_slice(input)?;

    match instruction {
        MetadataInstruction::CreateMetadataEntry {
            name,
            value,
            hashed_name,
        } => {
            msg!("Instruction: Create Metadata Entry");
            process_create_metadata_entry(program_id, accounts, name, value, hashed_name)
        }
        MetadataInstruction::UpdateMetadataEntry { value } => {
            msg!("Instruction: Update Metadata Entry");
            process_update_metadata_entry(program_id, accounts, value)
        }
        MetadataInstruction::DeleteMetadataEntry => {
            msg!("Instruction: Delete Metadata Entry");
            process_delete_metadata_entry(program_id, accounts)
        }
        MetadataInstruction::CreateVersionedIdl {
            effective_slot,
            idl_url,
            idl_hash,
            source_url,
            serialization,
            custom_layout_url,
            hashed_name,
        } => {
            msg!("Instruction: Create Versioned Idl");
            process_create_versioned_idl(
                program_id,
                accounts,
                effective_slot,
                idl_url,
                idl_hash,
                source_url,
                serialization,
                custom_layout_url,
                hashed_name,
            )
        }
        MetadataInstruction::UpdateVersionedIdl {
            idl_url,
            idl_hash,
            source_url,
            serialization,
            custom_layout_url,
        } => {
            msg!("Instruction: Update Versioned IDL");
            process_update_versioned_idl(
                program_id,
                accounts,
                idl_url,
                idl_hash,
                source_url,
                serialization,
                custom_layout_url,
            )
        }
    }
}

pub fn process_create_metadata_entry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    value: String,
    hashed_name: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let class_account_info = next_account_info(account_info_iter)?;
    let name_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let name_service_info = next_account_info(account_info_iter)?;

    if name.len() > MAX_NAME_LENGTH {
        return Err(MetadataError::NameTooLong.into());
    }

    if value.len() > MAX_VALUE_LENGTH {
        return Err(MetadataError::ValueTooLong.into());
    }

    if !target_program_authority_info.is_signer {
        msg!("The given program authority is not a signer.");
        return Err(MetadataError::ProgramAuthorityMustBeSigner.into());
    }

    assert_program_matches_program_data_address(
        target_program_info,
        target_program_program_data_info,
    )?;

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    let class_seeds = &[CLASS_PREFIX.as_bytes(), target_program_info.key.as_ref()];

    let (class_key, class_bump_seed) = Pubkey::find_program_address(class_seeds, program_id);

    let class_authority_signer_seeds = &[
        CLASS_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        &[class_bump_seed],
    ];

    if class_account_info.key != &class_key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    create_name_service_account(
        rent_sysvar_info,
        name_account_info,
        payer_account_info,
        name_service_info,
        system_account_info,
        class_account_info,
        class_authority_signer_seeds,
        &hashed_name,
        METADATA_ENTRY_SIZE,
    )?;

    let metadata_entry = MetadataEntry {
        account_type: AccountType::MetadataPairV1,
        name,
        value,
    };

    let mut serialized: Vec<u8> = vec![];

    metadata_entry.serialize(&mut serialized)?;

    update_name_service_account(
        name_account_info,
        class_account_info,
        name_service_info,
        class_authority_signer_seeds,
        &serialized,
    )?;

    Ok(())
}

pub fn process_update_metadata_entry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    value: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let class_account_info = next_account_info(account_info_iter)?;
    let name_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let name_service_info = next_account_info(account_info_iter)?;

    if value.len() > MAX_VALUE_LENGTH {
        return Err(MetadataError::ValueTooLong.into());
    }

    if !target_program_authority_info.is_signer {
        msg!("The given program authority is not a signer.");
        return Err(MetadataError::ProgramAuthorityMustBeSigner.into());
    }
    assert_program_matches_program_data_address(
        target_program_info,
        target_program_program_data_info,
    )?;

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    let class_seeds = &[CLASS_PREFIX.as_bytes(), target_program_info.key.as_ref()];

    let (class_key, class_bump_seed) = Pubkey::find_program_address(class_seeds, program_id);

    let class_authority_signer_seeds = &[
        CLASS_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        &[class_bump_seed],
    ];

    if class_account_info.key != &class_key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    let name_record_data = name_account_info.data.borrow();
    let mut metadata_entry: MetadataEntry = try_from_slice_unchecked(&name_record_data[96..])?;
    metadata_entry.value = value;

    let mut serialized: Vec<u8> = vec![];

    metadata_entry.serialize(&mut serialized)?;

    drop(name_record_data);

    update_name_service_account(
        name_account_info,
        class_account_info,
        name_service_info,
        class_authority_signer_seeds,
        &serialized,
    )?;

    Ok(())
}

pub fn process_delete_metadata_entry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let class_account_info = next_account_info(account_info_iter)?;
    let name_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let refund_info = next_account_info(account_info_iter)?;
    let name_service_info = next_account_info(account_info_iter)?;
    if !target_program_authority_info.is_signer {
        msg!("The given program authority is not a signer.");
        return Err(MetadataError::ProgramAuthorityMustBeSigner.into());
    }

    assert_program_matches_program_data_address(
        target_program_info,
        target_program_program_data_info,
    )?;

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    let class_seeds = &[CLASS_PREFIX.as_bytes(), target_program_info.key.as_ref()];

    let (class_key, class_bump_seed) = Pubkey::find_program_address(class_seeds, program_id);

    let class_authority_signer_seeds = &[
        CLASS_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        &[class_bump_seed],
    ];

    if class_account_info.key != &class_key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    delete_name_service_account(
        name_service_info,
        name_account_info,
        class_account_info,
        refund_info,
        class_authority_signer_seeds,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn process_create_versioned_idl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    effective_slot: u64,
    idl_url: String,
    idl_hash: [u8; 32],
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
    hashed_name: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let class_account_info = next_account_info(account_info_iter)?;
    let name_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let name_service_info = next_account_info(account_info_iter)?;

    if idl_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::IdlUrlTooLong.into());
    }

    if source_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::SourceUrlTooLong.into());
    }

    if let Some(custom_layout_url) = custom_layout_url.clone() {
        if custom_layout_url.len() > MAX_URL_LENGTH {
            return Err(MetadataError::CustomLayoutUrlTooLong.into());
        }
    }

    if !target_program_authority_info.is_signer {
        msg!("The given program authority is not a signer.");
        return Err(MetadataError::ProgramAuthorityMustBeSigner.into());
    }

    assert_program_matches_program_data_address(
        target_program_info,
        target_program_program_data_info,
    )?;

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    let class_seeds = &[CLASS_PREFIX.as_bytes(), target_program_info.key.as_ref()];

    let (class_key, class_bump_seed) = Pubkey::find_program_address(class_seeds, program_id);

    let class_authority_signer_seeds = &[
        CLASS_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        &[class_bump_seed],
    ];

    if class_account_info.key != &class_key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    create_name_service_account(
        rent_sysvar_info,
        name_account_info,
        payer_account_info,
        name_service_info,
        system_account_info,
        class_account_info,
        class_authority_signer_seeds,
        &Vec::from(hashed_name),
        VERSIONED_IDL_SIZE,
    )?;

    let idl_entry = VersionedIdl {
        account_type: AccountType::VersionedIdlV1,
        effective_slot,
        idl_url,
        idl_hash,
        source_url,
        serialization,
        custom_layout_url,
    };

    let mut serialized: Vec<u8> = vec![];
    idl_entry.serialize(&mut serialized)?;

    update_name_service_account(
        name_account_info,
        class_account_info,
        name_service_info,
        class_authority_signer_seeds,
        &serialized,
    )?;

    Ok(())
}

pub fn process_update_versioned_idl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    idl_url: String,
    idl_hash: [u8; 32],
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let class_account_info = next_account_info(account_info_iter)?;
    let name_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let name_service_info = next_account_info(account_info_iter)?;

    if idl_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::IdlUrlTooLong.into());
    }

    if source_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::SourceUrlTooLong.into());
    }

    if let Some(custom_layout_url) = custom_layout_url.clone() {
        if custom_layout_url.len() > MAX_URL_LENGTH {
            return Err(MetadataError::CustomLayoutUrlTooLong.into());
        }
    }

    if !target_program_authority_info.is_signer {
        msg!("The given program authority is not a signer.");
        return Err(MetadataError::ProgramAuthorityMustBeSigner.into());
    }

    assert_program_matches_program_data_address(
        target_program_info,
        target_program_program_data_info,
    )?;

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    let class_seeds = &[CLASS_PREFIX.as_bytes(), target_program_info.key.as_ref()];

    let (class_key, class_bump_seed) = Pubkey::find_program_address(class_seeds, program_id);

    let class_authority_signer_seeds = &[
        CLASS_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        &[class_bump_seed],
    ];

    if class_account_info.key != &class_key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    let name_record_data = name_account_info.data.borrow();
    let mut idl_entry: VersionedIdl = try_from_slice_unchecked(&name_record_data[96..])?;

    idl_entry.idl_url = idl_url;
    idl_entry.idl_hash = idl_hash.to_owned();
    idl_entry.serialization = serialization;
    idl_entry.source_url = source_url;
    idl_entry.custom_layout_url = custom_layout_url;

    let mut serialized: Vec<u8> = vec![];
    idl_entry.serialize(&mut serialized)?;

    drop(name_record_data);

    update_name_service_account(
        name_account_info,
        class_account_info,
        name_service_info,
        class_authority_signer_seeds,
        &serialized,
    )?;

    Ok(())
}
