use {
    crate::{
        error::MetadataError,
        instruction::MetadataInstruction,
        state::{
            AccountType, MetadataEntry, SerializationMethod, VersionedIdl, IDL_PREFIX,
            MAX_NAME_LENGTH, MAX_URL_LENGTH, MAX_VALUE_LENGTH, METADATA_ENTRY_SIZE,
            METADATA_PREFIX, VERSIONED_IDL_SIZE,
        },
        utils::{
            assert_program_authority_has_authority_over_program, create_or_allocate_account_raw,
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
        MetadataInstruction::CreateMetadataEntry { name, value } => {
            msg!("Instruction: Create Metadata Entry");
            process_create_metadata_entry(program_id, accounts, name, value)
        }
        MetadataInstruction::UpdateMetadataEntry { value } => {
            msg!("Instruction: Update Metadata Entry");
            process_update_metadata_entry(program_id, accounts, value)
        }
        MetadataInstruction::CreateVersionedIdl {
            effective_slot,
            idl_url,
            source_url,
            serialization,
            custom_layout_url,
        } => {
            msg!("Instruction: Transfer Update Authority");
            process_create_versioned_idl(
                program_id,
                accounts,
                effective_slot,
                idl_url,
                source_url,
                serialization,
                custom_layout_url,
            )
        }
        MetadataInstruction::UpdateVersionedIdl {
            idl_url,
            source_url,
            serialization,
            custom_layout_url,
        } => {
            msg!("Instruction: Update Versioned IDL");
            process_update_versioned_idl(
                program_id,
                accounts,
                idl_url,
                source_url,
                serialization,
                custom_layout_url,
            )
        }
        MetadataInstruction::TransferUpdateAuthority => {
            msg!("Instruction: Transfer Update Authority");
            process_transfer_update_authority(program_id, accounts)
        }
    }
}

pub fn process_create_metadata_entry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    value: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    if name.len() > MAX_NAME_LENGTH {
        return Err(MetadataError::NameTooLong.into());
    }

    if value.len() > MAX_VALUE_LENGTH {
        return Err(MetadataError::ValueTooLong.into());
    }

    let metadata_seeds = &[
        METADATA_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        name.as_ref(),
    ];

    let (metadata_key, metadata_bump_seed) =
        Pubkey::find_program_address(metadata_seeds, program_id);

    let metadata_authority_signer_seeds = &[
        METADATA_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        name.as_ref(),
        &[metadata_bump_seed],
    ];

    if metadata_account_info.key != &metadata_key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    create_or_allocate_account_raw(
        *program_id,
        metadata_account_info,
        rent_info,
        system_account_info,
        payer_account_info,
        METADATA_ENTRY_SIZE,
        metadata_authority_signer_seeds,
    )?;

    let mut metadata: MetadataEntry =
        try_from_slice_unchecked(&metadata_account_info.data.borrow())?;
    metadata.account_type = AccountType::MetadataPairV1;
    metadata.program_id = *target_program_info.key;
    metadata.name = name.to_owned();
    metadata.value = value.to_owned();
    metadata.update_authority = *update_authority_info.key;
    metadata.serialize(&mut *metadata_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_update_metadata_entry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    value: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    if value.len() > MAX_VALUE_LENGTH {
        return Err(MetadataError::ValueTooLong.into());
    }

    let mut metadata: MetadataEntry =
        try_from_slice_unchecked(&metadata_account_info.data.borrow())?;

    let metadata_seeds = &[
        METADATA_PREFIX.as_bytes(),
        metadata.program_id.as_ref(),
        metadata.name.as_ref(),
    ];

    let (metadata_key, _metadata_bump_seed) =
        Pubkey::find_program_address(metadata_seeds, program_id);

    if metadata_key != *metadata_account_info.key {
        return Err(MetadataError::InvalidMetadataAccount.into());
    }

    if metadata.update_authority != *update_authority_info.key {
        return Err(MetadataError::UpdateAuthorityIncorrect.into());
    }

    metadata.value = value.to_owned();
    metadata.serialize(&mut *metadata_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_create_versioned_idl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    effective_slot: u64,
    idl_url: String,
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_account_info = next_account_info(account_info_iter)?;
    let target_program_info = next_account_info(account_info_iter)?;
    let target_program_program_data_info = next_account_info(account_info_iter)?;
    let target_program_authority_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    if idl_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::IDLUrlTooLong.into());
    }

    if source_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::SourceUrlTooLong.into());
    }

    if let Some(custom_layout_url) = custom_layout_url.clone() {
        if custom_layout_url.len() > MAX_URL_LENGTH {
            return Err(MetadataError::CustomLayoutUrlTooLong.into());
        }
    }

    let effective_slot_bytes = effective_slot.to_le_bytes();

    let metadata_seeds = &[
        IDL_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        effective_slot_bytes.as_ref(),
    ];

    let (metadata_key, metadata_bump_seed) =
        Pubkey::find_program_address(metadata_seeds, program_id);

    let metadata_authority_signer_seeds = &[
        IDL_PREFIX.as_bytes(),
        target_program_info.key.as_ref(),
        effective_slot_bytes.as_ref(),
        &[metadata_bump_seed],
    ];

    if metadata_account_info.key != &metadata_key {
        return Err(MetadataError::InvalidIdlAccount.into());
    }

    assert_program_authority_has_authority_over_program(
        target_program_authority_info,
        target_program_program_data_info,
    )?;

    create_or_allocate_account_raw(
        *program_id,
        metadata_account_info,
        rent_info,
        system_account_info,
        payer_account_info,
        VERSIONED_IDL_SIZE,
        metadata_authority_signer_seeds,
    )?;

    let mut versioned_idl: VersionedIdl =
        try_from_slice_unchecked(&metadata_account_info.data.borrow())?;

    versioned_idl.account_type = AccountType::VersionedIdlV1;
    versioned_idl.program_id = *target_program_info.key;
    versioned_idl.idl_url = idl_url;
    versioned_idl.source_url = source_url;
    versioned_idl.serialization = serialization;
    versioned_idl.custom_layout_url = custom_layout_url;
    versioned_idl.update_authority = *update_authority_info.key;
    versioned_idl.serialize(&mut *metadata_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_update_versioned_idl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    idl_url: String,
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_account_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;

    if idl_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::IDLUrlTooLong.into());
    }

    if source_url.len() > MAX_URL_LENGTH {
        return Err(MetadataError::SourceUrlTooLong.into());
    }

    if let Some(custom_layout_url) = custom_layout_url.clone() {
        if custom_layout_url.len() > MAX_URL_LENGTH {
            return Err(MetadataError::CustomLayoutUrlTooLong.into());
        }
    }

    let mut versioned_idl: VersionedIdl =
        try_from_slice_unchecked(&metadata_account_info.data.borrow())?;

    let effective_slot_bytes = versioned_idl.effective_slot.to_le_bytes();

    let metadata_seeds = &[
        IDL_PREFIX.as_bytes(),
        versioned_idl.program_id.as_ref(),
        effective_slot_bytes.as_ref(),
    ];

    let (metadata_key, _metadata_bump_seed) =
        Pubkey::find_program_address(metadata_seeds, program_id);

    if metadata_key != *metadata_account_info.key {
        return Err(MetadataError::InvalidIdlAccount.into());
    }

    if versioned_idl.update_authority != *update_authority_info.key {
        return Err(MetadataError::UpdateAuthorityIncorrect.into());
    }

    versioned_idl.idl_url = idl_url.to_owned();
    versioned_idl.source_url = idl_url.to_owned();
    versioned_idl.serialization = serialization.to_owned();
    versioned_idl.custom_layout_url = custom_layout_url.to_owned();
    versioned_idl.serialize(&mut *metadata_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_transfer_update_authority(_: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let account_info = next_account_info(account_info_iter)?;
    let current_update_authority_info = next_account_info(account_info_iter)?;
    let new_update_authority_info = next_account_info(account_info_iter)?;

    if account_info.data_len() == METADATA_ENTRY_SIZE {
        let mut metadata: MetadataEntry = try_from_slice_unchecked(&account_info.data.borrow())?;

        if metadata.update_authority != *current_update_authority_info.key {
            return Err(MetadataError::UpdateAuthorityIncorrect.into());
        }

        metadata.update_authority = *new_update_authority_info.key;
        metadata.serialize(&mut *account_info.data.borrow_mut())?;
    } else {
        let mut versioned_idl: VersionedIdl =
            try_from_slice_unchecked(&account_info.data.borrow())?;

        if versioned_idl.update_authority != *current_update_authority_info.key {
            return Err(MetadataError::UpdateAuthorityIncorrect.into());
        }

        versioned_idl.update_authority = *new_update_authority_info.key;
        versioned_idl.serialize(&mut *account_info.data.borrow_mut())?;
    }

    Ok(())
}
