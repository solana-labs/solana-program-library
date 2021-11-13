use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use error::VoterWeightAddinError;
use spl_governance::{
    addins::voter_weight::{VoterWeightAccountType, VoterWeightRecord},
    state::token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
};
use spl_governance_tools::account::create_and_serialize_account;
use spl_governance::state::realm::{
    get_realm_data,get_realm_data_for_governing_token_mint, get_realm_data_for_authority
};

/// Processes Deposit instruction
pub fn process_add_voter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    address:&Pubkey
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_program_info = next_account_info(account_info_iter)?; // 0
    let realm_info = next_account_info(account_info_iter)?; // 1
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 2
    let token_owner_record_info = next_account_info(account_info_iter)?; // 3
    let voter_weight_record_info = next_account_info(account_info_iter)?; // 4
    let payer_info = next_account_info(account_info_iter)?; // 5
    let system_info = next_account_info(account_info_iter)?; // 6

    // check the realm authority address before create account voterWeightAddin
    let realm_data = get_realm_data_for_governing_token_mint(program_id, realm_info, governing_token_mint_info.key);
    if realm_data.unwrap().authority != payer_info.key {
        return Err(VoterWeightAddinError::CantAddVoterWeight.into());
    }

    let token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        governance_program_info.key,
        token_owner_record_info,
        realm_info.key,
        governing_token_mint_info.key,
    )?;

    // TODO: Custom deposit logic and validation goes here
    let voter_weight_record_data = VoterWeightRecord {
        account_type: VoterWeightAccountType::VoterWeightRecord,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        // changed to the user who cant to add
        governing_token_owner: address,
        voter_weight: amount,
        voter_weight_expiry: None,
    };

    create_and_serialize_account(
        payer_info,
        voter_weight_record_info,
        &voter_weight_record_data,
        program_id,
        system_info,
    )?;

    Ok(())
}
