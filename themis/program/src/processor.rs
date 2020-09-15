//! Themis program
use crate::{
    error::ThemisError,
    instruction::ThemisInstruction,
    state::{Policies, User},
};
use bn::{arith::U256, AffineG1, Fq, Group, G1};
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token::pack::Pack;

/// Decode an array of two U256's as a G1 point.
fn unpack_point(input: &[U256]) -> Result<G1, ProgramError> {
    use ThemisError::InvalidInstruction;

    let px = Fq::from_u256(input[0]).unwrap();
    let py = Fq::from_u256(input[1]).unwrap();
    let p = if px == Fq::zero() && py == Fq::zero() {
        G1::zero()
    } else {
        AffineG1::new(px, py)
            .map_err(|_| InvalidInstruction)?
            .into()
    };
    Ok(p)
}

/// Decode an array of four U256's as a pair of points.
fn unpack_points(input: &[U256]) -> Result<(G1, G1), ProgramError> {
    Ok((unpack_point(&input[0..2])?, unpack_point(&input[3..4])?))
}

/// Process the CalcualteAggregate instruction.
fn process_calculate_aggregate(
    user_info: &AccountInfo,
    policies_info: &AccountInfo,
    packed_interactions: &[[U256; 4]],
    packed_public_key: [U256; 2],
) -> Result<(), ProgramError> {
    let mut user = User::unpack(&user_info.data.borrow())?;
    let policies = Policies::unpack(&policies_info.data.borrow())?;
    let interactions = packed_interactions
        .iter()
        .map(|interaction| unpack_points(interaction))
        .collect::<Result<Vec<_>, _>>()?;
    let public_key = unpack_point(&packed_public_key)?;
    user.calculate_aggregate(&interactions, public_key, &policies.policies);
    User::pack(*user_info.data.borrow_mut())
}

/// Process the given transaction instruction
pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    account_infos: &'a [AccountInfo<'a>],
    input: &[u8],
) -> Result<(), ProgramError> {
    let account_infos_iter = &mut account_infos.iter();
    let instruction = ThemisInstruction::unpack(input)?;

    match instruction {
        ThemisInstruction::InitializeUserAccount => {
            let _user_info = next_account_info(account_infos_iter)?;
            //process_initialize_user_account(&user_info)
            Ok(())
        }
        ThemisInstruction::InitializePoliciesAccount { policies: _ } => {
            let _policies_info = next_account_info(account_infos_iter)?;
            //process_initialize_policies_account(&policies_info)
            Ok(())
        }
        ThemisInstruction::CalculateAggregate {
            encrypted_interactions,
            public_key,
        } => {
            let user_info = next_account_info(account_infos_iter)?;
            let policies_info = next_account_info(account_infos_iter)?;
            process_calculate_aggregate(
                &user_info,
                &policies_info,
                &encrypted_interactions,
                public_key,
            )
        }
        ThemisInstruction::SubmitProofDecryption {
            plaintext: _,
            announcement_g: _,
            announcement_ctx: _,
            response: _,
        } => {
            let _user_info = next_account_info(account_infos_iter)?;
            Ok(())
        }
        ThemisInstruction::RequestPayment {
            encrypted_aggregate: _,
            decrypted_aggregate: _,
            proof_correct_decryption: _,
        } => {
            let _user_info = next_account_info(account_infos_iter)?;
            Ok(())
        }
    }
}
