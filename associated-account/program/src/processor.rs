//! Program state processor

use crate::instruction::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    info,
    log::sol_log_compute_units,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};
use speedy::{self, Readable};

pub(crate) fn get_associated_address_and_bump_seed_with_id(
    primary_account_address: &Pubkey,
    associated_account_program_id: &Pubkey,
    additional_addresses: &[&Pubkey],
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    // Gross code to produce an `address_seeds` variable of type `Vec<&[u8]>`.  Any better
    // ideas?
    let mut address_seeds_vec = vec![
        primary_account_address.to_bytes(),
        associated_account_program_id.to_bytes(),
    ];
    address_seeds_vec.extend(additional_addresses.iter().map(|p| p.to_bytes()));

    let mut address_seeds: Vec<&[u8]> = Vec::with_capacity(address_seeds_vec.len());
    for seed in address_seeds_vec.iter() {
        address_seeds.push(seed);
    }

    Pubkey::find_program_address(&address_seeds, program_id)
}

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction_data = InstructionData::read_from_buffer(input).map_err(|err| {
        info!(&format!(
            "Error: Failed to decode instruction data: {}",
            err
        ));
        ProgramError::InvalidInstructionData
    })?;
    sol_log_compute_units();

    let account_info_iter = &mut accounts.iter();

    let associated_account_info = next_account_info(account_info_iter)?;
    let primary_account_info = next_account_info(account_info_iter)?;
    let associated_account_program_id_info = next_account_info(account_info_iter)?;
    let funder_info = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let additional_addresses = account_info_iter.map(|ai| ai.key).collect::<Vec<_>>();

    let (associated_address, bump_seed) = get_associated_address_and_bump_seed_with_id(
        &primary_account_info.key,
        &associated_account_program_id_info.key,
        &additional_addresses,
        program_id,
    );
    if associated_address != *associated_account_info.key {
        info!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    // Gross code to produce an `associated_account_signer_seeds` variable of type `Vec<&[u8]>`.  Any better
    // ideas?
    let mut address_seeds_vec = vec![
        primary_account_info.key.to_bytes(),
        associated_account_program_id_info.key.to_bytes(),
    ];
    address_seeds_vec.extend(additional_addresses.iter().map(|p| p.to_bytes()));

    let mut associated_account_signer_seeds: Vec<&[u8]> =
        Vec::with_capacity(address_seeds_vec.len());
    for seed in address_seeds_vec.iter() {
        associated_account_signer_seeds.push(seed);
    }
    let bump_seed_as_array = [bump_seed];
    associated_account_signer_seeds.push(&bump_seed_as_array);

    sol_log_compute_units();

    // Fund the associated account if necessary
    let required_lamports = instruction_data
        .lamports
        .saturating_sub(associated_account_info.lamports());
    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(
                &funder_info.key,
                associated_account_info.key,
                required_lamports,
            ),
            &[
                funder_info.clone(),
                associated_account_info.clone(),
                system_program.clone(),
            ],
        )?;
    }

    // Allocate space for the associated account
    invoke_signed(
        &system_instruction::allocate(associated_account_info.key, instruction_data.space),
        &[associated_account_info.clone(), system_program.clone()],
        &[&associated_account_signer_seeds],
    )?;

    // Assign the associated account to the desired program
    invoke_signed(
        &system_instruction::assign(
            associated_account_info.key,
            associated_account_program_id_info.key,
        ),
        &[associated_account_info.clone(), system_program.clone()],
        &[&associated_account_signer_seeds],
    )
}
