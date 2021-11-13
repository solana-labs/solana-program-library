use spl_governance::addins::voter_weight::{
    get_voter_weight_record_data_for_token_owner_record
};
/// Processes Deposit instruction
pub fn process_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
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

    let token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        governance_program_info.key,
        token_owner_record_info,
        realm_info.key,
        governing_token_mint_info.key,
    )?;

    // check token_owner_record_info have already created
    let voter_weight_is_added = get_voter_weight_record_data_for_token_owner_record(program_id, voter_weight_record_info, token_owner_record_info);
   
    // TODO: Custom deposit logic and validation goes here
    let voter_weight_record_data = VoterWeightRecord {
        account_type: VoterWeightAccountType::VoterWeightRecord,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        governing_token_owner: token_owner_record_data.governing_token_owner,
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
