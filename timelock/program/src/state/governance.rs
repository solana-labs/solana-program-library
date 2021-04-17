use crate::{
    state::{
        enums::GovernanceAccountType,
        enums::{ExecutionType, GovernanceType, VotingEntryRule},
    },
    utils::{pack_option_key, unpack_option_key},
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// max name length
pub const GOVERNANCE_NAME_LENGTH: usize = 32;
/// Timelock Config
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Governance {
    /// Account type
    pub account_type: GovernanceAccountType,
    /// Voting threshold in % required to tip the vote
    pub vote_threshold: u8,
    /// Execution type
    pub execution_type: ExecutionType,
    /// Timelock Type
    pub governance_type: GovernanceType,
    /// Voting entry rule
    pub voting_entry_rule: VotingEntryRule,
    /// Minimum slot time-distance from creation of proposal for an instruction to be placed
    pub minimum_slot_waiting_period: u64,
    /// Governance mint
    pub governance_mint: Pubkey,
    /// Council mint
    pub council_mint: Option<Pubkey>,
    /// Program ID that is tied to this config (optional)
    pub program: Pubkey,
    /// Time limit in slots for proposal to be open to voting
    pub time_limit: u64,
    /// Optional name
    pub name: [u8; GOVERNANCE_NAME_LENGTH],
    /// Running count of proposals
    pub count: u32,
}

impl Sealed for Governance {}
impl IsInitialized for Governance {
    fn is_initialized(&self) -> bool {
        self.account_type != GovernanceAccountType::Uninitialized
    }
}

/// Len of timelock config
pub const GOVERNANCE_LEN: usize =
    1 + 1 + 1 + 1 + 1 + 8 + 32 + 33 + 32 + 8 + GOVERNANCE_NAME_LENGTH + 4 + 295;

impl Pack for Governance {
    const LEN: usize = 1 + 1 + 1 + 1 + 1 + 8 + 32 + 33 + 32 + 8 + GOVERNANCE_NAME_LENGTH + 4 + 295;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, GOVERNANCE_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            account_type_value,
            vote_threshold,
            execution_type,
            governance_type,
            voting_entry_rule,
            minimum_slot_waiting_period,
            governance_mint,
            council_mint_option,
            program,
            time_limit,
            name,
            count,
            _padding,
        ) = array_refs![
            input,
            1,
            1,
            1,
            1,
            1,
            8,
            32,
            33,
            32,
            8,
            GOVERNANCE_NAME_LENGTH,
            4,
            295
        ];
        let account_type = u8::from_le_bytes(*account_type_value);
        let vote_threshold = u8::from_le_bytes(*vote_threshold);
        let execution_type = u8::from_le_bytes(*execution_type);
        let governance_type = u8::from_le_bytes(*governance_type);
        let voting_entry_rule = u8::from_le_bytes(*voting_entry_rule);
        let minimum_slot_waiting_period = u64::from_le_bytes(*minimum_slot_waiting_period);
        let time_limit = u64::from_le_bytes(*time_limit);
        let count = u32::from_le_bytes(*count);

        let account_type = match account_type {
            0 => GovernanceAccountType::Uninitialized,
            1 => GovernanceAccountType::Governance,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Self {
            account_type,
            vote_threshold,
            execution_type: match execution_type {
                0 => ExecutionType::Independent,
                _ => ExecutionType::Independent,
            },
            governance_type: match governance_type {
                0 => GovernanceType::Governance,
                _ => GovernanceType::Governance,
            },
            voting_entry_rule: match voting_entry_rule {
                0 => VotingEntryRule::Anytime,
                _ => VotingEntryRule::Anytime,
            },
            minimum_slot_waiting_period,
            governance_mint: Pubkey::new_from_array(*governance_mint),

            council_mint: unpack_option_key(council_mint_option)?,

            program: Pubkey::new_from_array(*program),
            time_limit,
            name: *name,
            count,
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, GOVERNANCE_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            account_type_value,
            vote_threshold,
            execution_type,
            governance_type,
            voting_entry_rule,
            minimum_slot_waiting_period,
            governance_mint,
            council_mint_option,
            program,
            time_limit,
            name,
            count,
            _padding,
        ) = mut_array_refs![
            output,
            1,
            1,
            1,
            1,
            1,
            8,
            32,
            33,
            32,
            8,
            GOVERNANCE_NAME_LENGTH,
            4,
            295
        ];
        *account_type_value = match self.account_type {
            GovernanceAccountType::Uninitialized => 0_u8,
            GovernanceAccountType::Governance => 1_u8,
            _ => panic!("Account type was invalid"),
        }
        .to_le_bytes();

        *vote_threshold = self.vote_threshold.to_le_bytes();

        *execution_type = match self.execution_type {
            ExecutionType::Independent => 0_u8,
        }
        .to_le_bytes();
        *governance_type = match self.governance_type {
            GovernanceType::Governance => 0_u8,
        }
        .to_le_bytes();
        *voting_entry_rule = match self.voting_entry_rule {
            VotingEntryRule::Anytime => 0_u8,
        }
        .to_le_bytes();
        *minimum_slot_waiting_period = self.minimum_slot_waiting_period.to_le_bytes();
        governance_mint.copy_from_slice(self.governance_mint.as_ref());

        pack_option_key(self.council_mint, council_mint_option);

        program.copy_from_slice(self.program.as_ref());
        *time_limit = self.time_limit.to_le_bytes();
        name.copy_from_slice(self.name.as_ref());
        *count = self.count.to_le_bytes();
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
