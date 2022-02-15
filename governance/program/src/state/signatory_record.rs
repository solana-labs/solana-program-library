//! Signatory Record

use borsh::maybestd::io::Write;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::borsh::try_from_slice_unchecked;
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::{error::GovernanceError, PROGRAM_AUTHORITY_SEED};

use crate::state::enums::GovernanceAccountType;

use super::legacy::SignatoryRecordV1;

/// Account PDA seeds: ['governance', proposal, signatory]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct SignatoryRecordV2 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Proposal the signatory is assigned for
    pub proposal: Pubkey,

    /// The account of the signatory who can sign off the proposal
    pub signatory: Pubkey,

    /// Indicates whether the signatory signed off the proposal
    pub signed_off: bool,

    /// Reserved space for versions v2 and onwards
    /// Note: This space won't be available to v1 accounts until runtime supports resizing
    pub reserved_v2: [u8; 8],
}

impl AccountMaxSize for SignatoryRecordV2 {}

impl IsInitialized for SignatoryRecordV2 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::SignatoryRecordV2
    }
}

impl SignatoryRecordV2 {
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

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::SignatoryRecordV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::SignatoryRecordV1 {
            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 8] {
                panic!("Extended data not supported by SignatoryRecordV1")
            }

            let signatory_record_data_v1 = SignatoryRecordV1 {
                account_type: self.account_type,
                proposal: self.proposal,
                signatory: self.signatory,
                signed_off: self.signed_off,
            };

            BorshSerialize::serialize(&signatory_record_data_v1, writer)?;
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
pub fn get_signatory_record_address<'a>(
    program_id: &Pubkey,
    proposal: &'a Pubkey,
    signatory: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_signatory_record_address_seeds(proposal, signatory),
        program_id,
    )
    .0
}

/// Deserializes SignatoryRecord account and checks owner program
pub fn get_signatory_record_data(
    program_id: &Pubkey,
    signatory_record_info: &AccountInfo,
) -> Result<SignatoryRecordV2, ProgramError> {
    let account_type: GovernanceAccountType =
        try_from_slice_unchecked(&signatory_record_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    if account_type == GovernanceAccountType::SignatoryRecordV1 {
        let signatory_record_data_v1 =
            get_account_data::<SignatoryRecordV1>(program_id, signatory_record_info)?;

        return Ok(SignatoryRecordV2 {
            account_type,

            proposal: signatory_record_data_v1.proposal,
            signatory: signatory_record_data_v1.signatory,
            signed_off: signatory_record_data_v1.signed_off,

            // Add the extra reserved_v2 padding
            reserved_v2: [0; 8],
        });
    }

    get_account_data::<SignatoryRecordV2>(program_id, signatory_record_info)
}

/// Deserializes SignatoryRecord  and validates its PDA
pub fn get_signatory_record_data_for_seeds(
    program_id: &Pubkey,
    signatory_record_info: &AccountInfo,
    proposal: &Pubkey,
    signatory: &Pubkey,
) -> Result<SignatoryRecordV2, ProgramError> {
    let (signatory_record_address, _) = Pubkey::find_program_address(
        &get_signatory_record_address_seeds(proposal, signatory),
        program_id,
    );

    if signatory_record_address != *signatory_record_info.key {
        return Err(GovernanceError::InvalidSignatoryAddress.into());
    }

    get_signatory_record_data(program_id, signatory_record_info)
}
