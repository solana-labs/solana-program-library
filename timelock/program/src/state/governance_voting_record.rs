use crate::state::enums::GovernanceAccountType;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// STRUCT VERSION
pub const GOVERNANCE_VOTING_RECORD_VERSION: u8 = 1;
/// Governance Voting Record
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GovernanceVotingRecord {
    /// Account type
    pub account_type: GovernanceAccountType,

    /// proposal
    pub proposal: Pubkey,
    /// owner
    pub owner: Pubkey,

    /// How many votes were unspent
    pub undecided_count: u64,
    /// How many votes were spent yes
    pub yes_count: u64,
    /// How many votes were spent no
    pub no_count: u64,
}

impl Sealed for GovernanceVotingRecord {}
impl IsInitialized for GovernanceVotingRecord {
    fn is_initialized(&self) -> bool {
        self.account_type != GovernanceAccountType::Uninitialized
    }
}

/// Len of governance voting record
pub const GOVERNANCE_VOTING_RECORD_LEN: usize = 32 + 32 + 1 + 8 + 8 + 8 + 100;
impl Pack for GovernanceVotingRecord {
    const LEN: usize = 32 + 32 + 1 + 8 + 8 + 8 + 100;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, GOVERNANCE_VOTING_RECORD_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (account_type_value, proposal, owner, undecided_count, yes_count, no_count, _padding) =
            array_refs![input, 1, 32, 32, 8, 8, 8, 100];
        let account_type = u8::from_le_bytes(*account_type_value);
        let undecided_count = u64::from_le_bytes(*undecided_count);
        let yes_count = u64::from_le_bytes(*yes_count);
        let no_count = u64::from_le_bytes(*no_count);

        let account_type = match account_type {
            0 => GovernanceAccountType::Uninitialized,
            4 => GovernanceAccountType::VoteRecord,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Self {
            account_type,
            proposal: Pubkey::new_from_array(*proposal),
            owner: Pubkey::new_from_array(*owner),

            undecided_count,
            yes_count,
            no_count,
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, GOVERNANCE_VOTING_RECORD_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (account_type_value, proposal, owner, undecided_count, yes_count, no_count, _padding) =
            mut_array_refs![output, 1, 32, 32, 8, 8, 8, 100];

        *account_type_value = match self.account_type {
            GovernanceAccountType::Uninitialized => 0_u8,
            GovernanceAccountType::VoteRecord => 4_u8,
            _ => panic!("Account type was invalid"),
        }
        .to_le_bytes();

        proposal.copy_from_slice(self.proposal.as_ref());
        owner.copy_from_slice(self.owner.as_ref());

        *undecided_count = self.undecided_count.to_le_bytes();
        *yes_count = self.yes_count.to_le_bytes();
        *no_count = self.no_count.to_le_bytes();
    }

    fn get_packed_len() -> usize {
        Self::LEN
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError>
    where
        Self: IsInitialized,
    {
        let value = Self::unpack_unchecked(input)?;
        if value.is_initialized() {
            Ok(value)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    }

    fn unpack_unchecked(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Self::unpack_from_slice(input)
    }

    fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        src.pack_into_slice(dst);
        Ok(())
    }
}
