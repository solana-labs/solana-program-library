//! Program state processor

use crate::{instruction::*, state::*, *};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    feature::{self, Feature},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = FeatureProposalInstruction::unpack_from_slice(input)?;
    let account_info_iter = &mut accounts.iter();

    match instruction {
        FeatureProposalInstruction::Propose {
            tokens_to_mint,
            acceptance_criteria,
        } => {
            msg!("FeatureProposalInstruction::Propose");

            let funder_info = next_account_info(account_info_iter)?;
            let feature_proposal_info = next_account_info(account_info_iter)?;
            let mint_info = next_account_info(account_info_iter)?;
            let distributor_token_info = next_account_info(account_info_iter)?;
            let acceptance_token_info = next_account_info(account_info_iter)?;
            let feature_id_info = next_account_info(account_info_iter)?;
            let system_program_info = next_account_info(account_info_iter)?;
            let spl_token_program_info = next_account_info(account_info_iter)?;
            let rent_sysvar_info = next_account_info(account_info_iter)?;
            let rent = &Rent::from_account_info(rent_sysvar_info)?;

            let (mint_address, mint_bump_seed) =
                get_mint_address_with_seed(feature_proposal_info.key);
            if mint_address != *mint_info.key {
                msg!("Error: mint address derivation mismatch");
                return Err(ProgramError::InvalidArgument);
            }

            let (distributor_token_address, distributor_token_bump_seed) =
                get_distributor_token_address_with_seed(feature_proposal_info.key);
            if distributor_token_address != *distributor_token_info.key {
                msg!("Error: distributor token address derivation mismatch");
                return Err(ProgramError::InvalidArgument);
            }

            let (acceptance_token_address, acceptance_token_bump_seed) =
                get_acceptance_token_address_with_seed(feature_proposal_info.key);
            if acceptance_token_address != *acceptance_token_info.key {
                msg!("Error: acceptance token address derivation mismatch");
                return Err(ProgramError::InvalidArgument);
            }

            let (feature_id_address, feature_id_bump_seed) =
                get_feature_id_address_with_seed(feature_proposal_info.key);
            if feature_id_address != *feature_id_info.key {
                msg!("Error: feature-id address derivation mismatch");
                return Err(ProgramError::InvalidArgument);
            }

            let mint_signer_seeds: &[&[_]] = &[
                &feature_proposal_info.key.to_bytes(),
                br"mint",
                &[mint_bump_seed],
            ];

            let distributor_token_signer_seeds: &[&[_]] = &[
                &feature_proposal_info.key.to_bytes(),
                br"distributor",
                &[distributor_token_bump_seed],
            ];

            let acceptance_token_signer_seeds: &[&[_]] = &[
                &feature_proposal_info.key.to_bytes(),
                br"acceptance",
                &[acceptance_token_bump_seed],
            ];

            let feature_id_signer_seeds: &[&[_]] = &[
                &feature_proposal_info.key.to_bytes(),
                br"feature-id",
                &[feature_id_bump_seed],
            ];

            msg!("Creating feature proposal account");
            invoke(
                &system_instruction::create_account(
                    funder_info.key,
                    feature_proposal_info.key,
                    1.max(rent.minimum_balance(FeatureProposal::get_packed_len())),
                    FeatureProposal::get_packed_len() as u64,
                    program_id,
                ),
                &[
                    funder_info.clone(),
                    feature_proposal_info.clone(),
                    system_program_info.clone(),
                ],
            )?;
            FeatureProposal::Pending(acceptance_criteria)
                .pack_into_slice(&mut feature_proposal_info.data.borrow_mut());

            msg!("Creating mint");
            invoke_signed(
                &system_instruction::create_account(
                    funder_info.key,
                    mint_info.key,
                    1.max(rent.minimum_balance(spl_token::state::Mint::get_packed_len())),
                    spl_token::state::Mint::get_packed_len() as u64,
                    &spl_token::id(),
                ),
                &[
                    funder_info.clone(),
                    mint_info.clone(),
                    system_program_info.clone(),
                ],
                &[mint_signer_seeds],
            )?;

            msg!("Initializing mint");
            invoke(
                &spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    mint_info.key,
                    mint_info.key,
                    None,
                    spl_token::native_mint::DECIMALS,
                )?,
                &[
                    mint_info.clone(),
                    spl_token_program_info.clone(),
                    rent_sysvar_info.clone(),
                ],
            )?;

            msg!("Creating distributor token account");
            invoke_signed(
                &system_instruction::create_account(
                    funder_info.key,
                    distributor_token_info.key,
                    1.max(rent.minimum_balance(spl_token::state::Account::get_packed_len())),
                    spl_token::state::Account::get_packed_len() as u64,
                    &spl_token::id(),
                ),
                &[
                    funder_info.clone(),
                    distributor_token_info.clone(),
                    system_program_info.clone(),
                ],
                &[distributor_token_signer_seeds],
            )?;

            msg!("Initializing distributor token account");
            invoke(
                &spl_token::instruction::initialize_account(
                    &spl_token::id(),
                    distributor_token_info.key,
                    mint_info.key,
                    feature_proposal_info.key,
                )?,
                &[
                    distributor_token_info.clone(),
                    spl_token_program_info.clone(),
                    rent_sysvar_info.clone(),
                    feature_proposal_info.clone(),
                    mint_info.clone(),
                ],
            )?;

            msg!("Creating acceptance token account");
            invoke_signed(
                &system_instruction::create_account(
                    funder_info.key,
                    acceptance_token_info.key,
                    1.max(rent.minimum_balance(spl_token::state::Account::get_packed_len())),
                    spl_token::state::Account::get_packed_len() as u64,
                    &spl_token::id(),
                ),
                &[
                    funder_info.clone(),
                    acceptance_token_info.clone(),
                    system_program_info.clone(),
                ],
                &[acceptance_token_signer_seeds],
            )?;

            msg!("Initializing acceptance token account");
            invoke(
                &spl_token::instruction::initialize_account(
                    &spl_token::id(),
                    acceptance_token_info.key,
                    mint_info.key,
                    feature_proposal_info.key,
                )?,
                &[
                    acceptance_token_info.clone(),
                    spl_token_program_info.clone(),
                    rent_sysvar_info.clone(),
                    feature_proposal_info.clone(),
                    mint_info.clone(),
                ],
            )?;
            invoke(
                &spl_token::instruction::set_authority(
                    &spl_token::id(),
                    acceptance_token_info.key,
                    Some(feature_proposal_info.key),
                    spl_token::instruction::AuthorityType::CloseAccount,
                    feature_proposal_info.key,
                    &[],
                )?,
                &[
                    spl_token_program_info.clone(),
                    acceptance_token_info.clone(),
                    feature_proposal_info.clone(),
                ],
            )?;
            invoke(
                &spl_token::instruction::set_authority(
                    &spl_token::id(),
                    acceptance_token_info.key,
                    Some(program_id),
                    spl_token::instruction::AuthorityType::AccountOwner,
                    feature_proposal_info.key,
                    &[],
                )?,
                &[
                    spl_token_program_info.clone(),
                    acceptance_token_info.clone(),
                    feature_proposal_info.clone(),
                ],
            )?;

            // Mint `tokens_to_mint` tokens into `distributor_token_account` owned by
            // `feature_proposal`
            msg!("Minting {} tokens", tokens_to_mint);
            invoke_signed(
                &spl_token::instruction::mint_to(
                    &spl_token::id(),
                    mint_info.key,
                    distributor_token_info.key,
                    mint_info.key,
                    &[],
                    tokens_to_mint,
                )?,
                &[
                    mint_info.clone(),
                    distributor_token_info.clone(),
                    spl_token_program_info.clone(),
                ],
                &[mint_signer_seeds],
            )?;

            // Fully fund the feature id account so the `Tally` instruction will not require any
            // lamports from the caller
            msg!("Funding feature id account");
            invoke(
                &system_instruction::transfer(
                    funder_info.key,
                    feature_id_info.key,
                    1.max(rent.minimum_balance(Feature::size_of())),
                ),
                &[
                    funder_info.clone(),
                    feature_id_info.clone(),
                    system_program_info.clone(),
                ],
            )?;

            msg!("Allocating feature id account");
            invoke_signed(
                &system_instruction::allocate(feature_id_info.key, Feature::size_of() as u64),
                &[feature_id_info.clone(), system_program_info.clone()],
                &[feature_id_signer_seeds],
            )?;
        }

        FeatureProposalInstruction::Tally => {
            msg!("FeatureProposalInstruction::Tally");

            let feature_proposal_info = next_account_info(account_info_iter)?;
            let feature_proposal_state =
                FeatureProposal::unpack_from_slice(&feature_proposal_info.data.borrow())?;

            match feature_proposal_state {
                FeatureProposal::Pending(acceptance_criteria) => {
                    let acceptance_token_info = next_account_info(account_info_iter)?;
                    let feature_id_info = next_account_info(account_info_iter)?;
                    let system_program_info = next_account_info(account_info_iter)?;
                    let clock_sysvar_info = next_account_info(account_info_iter)?;
                    let clock = &Clock::from_account_info(clock_sysvar_info)?;

                    // Re-derive the acceptance token and feature id program addresses to confirm
                    // the caller provided the correct addresses
                    let acceptance_token_address =
                        get_acceptance_token_address(feature_proposal_info.key);
                    if acceptance_token_address != *acceptance_token_info.key {
                        msg!("Error: acceptance token address derivation mismatch");
                        return Err(ProgramError::InvalidArgument);
                    }

                    let (feature_id_address, feature_id_bump_seed) =
                        get_feature_id_address_with_seed(feature_proposal_info.key);
                    if feature_id_address != *feature_id_info.key {
                        msg!("Error: feature-id address derivation mismatch");
                        return Err(ProgramError::InvalidArgument);
                    }

                    let feature_id_signer_seeds: &[&[_]] = &[
                        &feature_proposal_info.key.to_bytes(),
                        br"feature-id",
                        &[feature_id_bump_seed],
                    ];

                    if clock.unix_timestamp >= acceptance_criteria.deadline {
                        msg!("Feature proposal expired");
                        FeatureProposal::Expired
                            .pack_into_slice(&mut feature_proposal_info.data.borrow_mut());
                        return Ok(());
                    }

                    msg!("Unpacking acceptance token account");
                    let acceptance_token =
                        spl_token::state::Account::unpack(&acceptance_token_info.data.borrow())?;

                    msg!(
                            "Feature proposal has received {} tokens, and {} tokens required for acceptance",
                            acceptance_token.amount, acceptance_criteria.tokens_required
                        );
                    if acceptance_token.amount < acceptance_criteria.tokens_required {
                        msg!("Activation threshold has not been reached");
                        return Ok(());
                    }

                    msg!("Assigning feature id account");
                    invoke_signed(
                        &system_instruction::assign(feature_id_info.key, &feature::id()),
                        &[feature_id_info.clone(), system_program_info.clone()],
                        &[feature_id_signer_seeds],
                    )?;

                    msg!("Feature proposal accepted");
                    FeatureProposal::Accepted {
                        tokens_upon_acceptance: acceptance_token.amount,
                    }
                    .pack_into_slice(&mut feature_proposal_info.data.borrow_mut());
                }
                _ => {
                    msg!("Error: feature proposal account not in the pending state");
                    return Err(ProgramError::InvalidAccountData);
                }
            }
        }
    }

    Ok(())
}
