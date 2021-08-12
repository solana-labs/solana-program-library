//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::{
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType,
        realm::get_realm_data,
        token_owner_record::{
            get_token_owner_record_address_seeds, get_token_owner_record_data_for_seeds,
            TokenOwnerRecord,
        },
    },
    tools::{
        account::create_and_serialize_account_signed,
        spl_token::{
            get_spl_token_amount, get_spl_token_mint, get_spl_token_owner, transfer_spl_tokens,
        },
    },
};

/// Processes DepositGoverningTokens instruction
pub fn process_deposit_governing_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_holding_info = next_account_info(account_info_iter)?; // 1
    let governing_token_source_info = next_account_info(account_info_iter)?; // 2
    let governing_token_owner_info = next_account_info(account_info_iter)?; // 3
    let governing_token_transfer_authority_info = next_account_info(account_info_iter)?; // 4
    let token_owner_record_info = next_account_info(account_info_iter)?; // 5
    let payer_info = next_account_info(account_info_iter)?; // 6
    let system_info = next_account_info(account_info_iter)?; // 7
    let spl_token_info = next_account_info(account_info_iter)?; // 8

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 9
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    let realm_data = get_realm_data(program_id, realm_info)?;
    let governing_token_mint = get_spl_token_mint(governing_token_holding_info)?;

    realm_data.assert_is_valid_governing_token_mint_and_holding(
        program_id,
        realm_info.key,
        &governing_token_mint,
        governing_token_holding_info.key,
    )?;

    let amount = get_spl_token_amount(governing_token_source_info)?;

    transfer_spl_tokens(
        governing_token_source_info,
        governing_token_holding_info,
        governing_token_transfer_authority_info,
        amount,
        spl_token_info,
    )?;

    let token_owner_record_address_seeds = get_token_owner_record_address_seeds(
        realm_info.key,
        &governing_token_mint,
        governing_token_owner_info.key,
    );

    if token_owner_record_info.data_is_empty() {
        // Deposited tokens can only be withdrawn by the owner so let's make sure the owner signed the transaction
        let governing_token_owner = get_spl_token_owner(governing_token_source_info)?;

        if !(governing_token_owner == *governing_token_owner_info.key
            && governing_token_owner_info.is_signer)
        {
            return Err(GovernanceError::GoverningTokenOwnerMustSign.into());
        }

        let token_owner_record_data = TokenOwnerRecord {
            account_type: GovernanceAccountType::TokenOwnerRecord,
            realm: *realm_info.key,
            governing_token_owner: *governing_token_owner_info.key,
            governing_token_deposit_amount: amount,
            governing_token_mint,
            governance_delegate: None,
            unrelinquished_votes_count: 0,
            total_votes_count: 0,
            outstanding_proposal_count: 0,
            reserved: [0; 7],
        };

        create_and_serialize_account_signed(
            payer_info,
            token_owner_record_info,
            &token_owner_record_data,
            &token_owner_record_address_seeds,
            program_id,
            system_info,
            rent,
        )?;
    } else {
        let mut token_owner_record_data = get_token_owner_record_data_for_seeds(
            program_id,
            token_owner_record_info,
            &token_owner_record_address_seeds,
        )?;

        token_owner_record_data.governing_token_deposit_amount = token_owner_record_data
            .governing_token_deposit_amount
            .checked_add(amount)
            .unwrap();

        token_owner_record_data.serialize(&mut *token_owner_record_info.data.borrow_mut())?;
    }

    Ok(())
}
