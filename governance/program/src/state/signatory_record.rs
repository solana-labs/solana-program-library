//! Signatory Record

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    id,
    tools::account::{deserialize_account, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use crate::state::enums::GovernanceAccountType;

/// Account PDA seeds: ['governance', proposal, signatory]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct SignatoryRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,
    /// Proposal the signatory is assigned for
    pub proposal: Pubkey,
    /// The account of the signatory who can sign off the proposal
    pub signatory: Pubkey,
    /// Indicates whether the signatory signed off the proposal
    pub signed_off: bool,
}

impl AccountMaxSize for SignatoryRecord {}

impl IsInitialized for SignatoryRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::SignatoryRecord
    }
}

impl SignatoryRecord {
    /// Checks signatory hasn't signed off yet and is transaction signer
    pub fn assert_can_sign_off(&self, signatory_info: &AccountInfo) -> Result<(), ProgramError> {
        if self.signed_off {
            return Err(GovernanceError::SignatoryAlreadySignedOff.into());
        }

        if !signatory_info.is_signer {
            return Err(GovernanceError::SignatoryMustSign.into());
        }

        Ok(())
    }

    /// Checks signatory can be removed from Proposal
    pub fn assert_can_remove_signatory(&self) -> Result<(), ProgramError> {
        if self.signed_off {
            return Err(GovernanceError::SignatoryAlreadySignedOff.into());
        }

        Ok(())
    }
}

/// Returns SignatoryRecord PDA seeds
pub fn get_signatory_record_address_seeds<'a>(
    proposal: &'a Pubkey,
    signatory: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        PROGRAM_AUTHORITY_SEED,
        proposal.as_ref(),
        signatory.as_ref(),
    ]
}

/// Returns SignatoryRecord PDA address
pub fn get_signatory_record_address<'a>(proposal: &'a Pubkey, signatory: &'a Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &get_signatory_record_address_seeds(proposal, signatory),
        &id(),
    )
    .0
}

/// Deserializes SignatoryRecord account and checks owner program
pub fn deserialize_signatory_record_raw(
    signatory_record_info: &AccountInfo,
) -> Result<SignatoryRecord, ProgramError> {
    deserialize_account::<SignatoryRecord>(signatory_record_info, &id())
}

/// Deserializes SignatoryRecord  and validates its PDA
pub fn deserialize_signatory_record(
    signatory_record_info: &AccountInfo,
    proposal: &Pubkey,
    signatory: &Pubkey,
) -> Result<SignatoryRecord, ProgramError> {
    let (signatory_record_address, _) = Pubkey::find_program_address(
        &get_signatory_record_address_seeds(proposal, signatory),
        &id(),
    );

    if signatory_record_address != *signatory_record_info.key {
        return Err(GovernanceError::InvalidSignatoryAddress.into());
    }

    deserialize_signatory_record_raw(signatory_record_info)
}
