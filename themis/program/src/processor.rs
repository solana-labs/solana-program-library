//! Themis program
use crate::{
    instruction::ThemisInstruction,
    state::{Policies, User},
};
use bincode::{deserialize, serialize_into};
use curve25519_dalek::ristretto::RistrettoPoint;
use elgamal_ristretto::public::PublicKey;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    pubkey::Pubkey,
};

fn process_calculate_aggregate(
    encrypted_interactions: &[(RistrettoPoint, RistrettoPoint)],
    public_key: PublicKey,
    user_info: &AccountInfo,
    policies_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let mut user: User = deserialize(&user_info.data.borrow()).unwrap();
    let policies: Policies = deserialize(&policies_info.data.borrow()).unwrap();
    user.calculate_aggregate(
        encrypted_interactions,
        public_key.get_point(),
        &policies.scalars,
    );
    serialize_into(&mut *user_info.data.borrow_mut(), &user).unwrap();
    Ok(())
}

/// Process the given transaction instruction
pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    account_infos: &'a [AccountInfo<'a>],
    input: &[u8],
) -> Result<(), ProgramError> {
    let account_infos_iter = &mut account_infos.iter();
    let instruction = deserialize(input).unwrap();

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
                &encrypted_interactions,
                public_key,
                &user_info,
                &policies_info,
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
