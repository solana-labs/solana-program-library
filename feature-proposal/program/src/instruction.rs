//! Program instructions

use crate::{state::AcceptanceCriteria, *};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
    sysvar,
};

/// Instructions supported by the Feature Proposal program
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub enum FeatureProposalInstruction {
    /// Propose a new feature.
    ///
    /// This instruction will create a variety of accounts to support the feature proposal, all
    /// funded by account 0:
    /// * A new token mint with a supply of `tokens_to_mint`, owned by the program and never
    ///   modified again
    /// * A new "distributor" token account that holds the total supply, owned by account 0.
    /// * A new "acceptance" token account that holds 0 tokens, owned by the program.  Tokens
    ///   transfers to this address are irrevocable and permanent.
    /// * A new feature id account that has been funded and allocated (as described in
    ///  `solana_program::feature`)
    ///
    /// On successful execution of the instruction, the feature proposer is expected to distribute
    /// the tokens in the distributor token account out to all participating parties.
    ///
    /// Based on the provided acceptance criteria, if `AcceptanceCriteria::tokens_required`
    /// tokens are transferred into the acceptance token account before
    /// `AcceptanceCriteria::deadline` then the proposal is eligible to be accepted.
    ///
    /// The `FeatureProposalInstruction::Tally` instruction must be executed, by any party, to
    /// complete the feature acceptance process.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable,signer]` Funding account (must be a system account)
    /// 1. `[writeable,signer]` Unallocated feature proposal account to create
    /// 2. `[writeable]` Token mint address from `get_mint_address`
    /// 3. `[writeable]` Distributor token account address from `get_distributor_token_address`
    /// 4. `[writeable]` Acceptance token account address from `get_acceptance_token_address`
    /// 5. `[writeable]` Feature id account address from `get_feature_id_address`
    /// 6. `[]` System program
    /// 7. `[]` SPL Token program
    /// 8. `[]` Rent sysvar
    ///
    Propose {
        /// Total number of tokens to mint for this proposal
        #[allow(dead_code)] // not dead code..
        tokens_to_mint: u64,

        /// Criteria for how this proposal may be activated
        #[allow(dead_code)] // not dead code..
        acceptance_criteria: AcceptanceCriteria,
    },

    /// `Tally` is a permission-less instruction to check the acceptance criteria for the feature
    /// proposal, which may result in:
    /// * No action
    /// * Feature proposal acceptance
    /// * Feature proposal expiration
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Feature proposal account
    /// 1. `[]` Acceptance token account address from `get_acceptance_token_address`
    /// 2. `[writeable]` Derived feature id account address from `get_feature_id_address`
    /// 3. `[]` System program
    /// 4. `[]` Clock sysvar
    Tally,
}

impl Sealed for FeatureProposalInstruction {}
impl Pack for FeatureProposalInstruction {
    const LEN: usize = 25; // see `test_get_packed_len()` for justification of "18"

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.pack_into_vec();
        dst[..data.len()].copy_from_slice(&data);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!(
                "Error: failed to deserialize feature proposal instruction: {}",
                err
            );
            ProgramError::InvalidInstructionData
        })
    }
}

impl FeatureProposalInstruction {
    fn pack_into_vec(&self) -> Vec<u8> {
        self.try_to_vec().expect("try_to_vec")
    }
}

/// Create a `FeatureProposalInstruction::Propose` instruction
pub fn propose(
    funding_address: &Pubkey,
    feature_proposal_address: &Pubkey,
    tokens_to_mint: u64,
    acceptance_criteria: AcceptanceCriteria,
) -> Instruction {
    let mint_address = get_mint_address(feature_proposal_address);
    let distributor_token_address = get_distributor_token_address(feature_proposal_address);
    let acceptance_token_address = get_acceptance_token_address(feature_proposal_address);
    let feature_id_address = get_feature_id_address(feature_proposal_address);

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funding_address, true),
            AccountMeta::new(*feature_proposal_address, true),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(distributor_token_address, false),
            AccountMeta::new(acceptance_token_address, false),
            AccountMeta::new(feature_id_address, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: FeatureProposalInstruction::Propose {
            tokens_to_mint,
            acceptance_criteria,
        }
        .pack_into_vec(),
    }
}

/// Create a `FeatureProposalInstruction::Tally` instruction
pub fn tally(feature_proposal_address: &Pubkey) -> Instruction {
    let acceptance_token_address = get_acceptance_token_address(feature_proposal_address);
    let feature_id_address = get_feature_id_address(feature_proposal_address);

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*feature_proposal_address, false),
            AccountMeta::new_readonly(acceptance_token_address, false),
            AccountMeta::new(feature_id_address, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: FeatureProposalInstruction::Tally.pack_into_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_packed_len() {
        assert_eq!(
            FeatureProposalInstruction::get_packed_len(),
            solana_program::borsh::get_packed_len::<FeatureProposalInstruction>()
        )
    }

    #[test]
    fn test_serialize_bytes() {
        assert_eq!(
            FeatureProposalInstruction::Tally.try_to_vec().unwrap(),
            vec![1]
        );

        assert_eq!(
            FeatureProposalInstruction::Propose {
                tokens_to_mint: 42,
                acceptance_criteria: AcceptanceCriteria {
                    tokens_required: 0xdeadbeefdeadbeef,
                    deadline: -1,
                }
            }
            .try_to_vec()
            .unwrap(),
            vec![
                0, 42, 0, 0, 0, 0, 0, 0, 0, 239, 190, 173, 222, 239, 190, 173, 222, 255, 255, 255,
                255, 255, 255, 255, 255
            ]
        );
    }

    #[test]
    fn test_serialize_large_slice() {
        let mut dst = vec![0xff; 4];
        FeatureProposalInstruction::Tally.pack_into_slice(&mut dst);

        // Extra bytes (0xff) ignored
        assert_eq!(dst, vec![1, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn state_deserialize_invalid() {
        assert_eq!(
            FeatureProposalInstruction::unpack_from_slice(&[1]),
            Ok(FeatureProposalInstruction::Tally),
        );

        // Extra bytes (0xff) ignored...
        assert_eq!(
            FeatureProposalInstruction::unpack_from_slice(&[1, 0xff, 0xff, 0xff]),
            Ok(FeatureProposalInstruction::Tally),
        );

        assert_eq!(
            FeatureProposalInstruction::unpack_from_slice(&[2]),
            Err(ProgramError::InvalidInstructionData),
        );
    }
}
