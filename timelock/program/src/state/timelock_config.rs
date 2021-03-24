use super::enums::{ConsensusAlgorithm, ExecutionType, TimelockType, VotingEntryRule};
use super::UNINITIALIZED_VERSION;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// STRUCT VERSION
pub const TIMELOCK_CONFIG_VERSION: u8 = 1;
/// max name length
pub const CONFIG_NAME_LENGTH: usize = 32;
/// Timelock Config
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockConfig {
    ///version
    pub version: u8,
    /// Consensus Algorithm
    pub consensus_algorithm: ConsensusAlgorithm,
    /// Execution type
    pub execution_type: ExecutionType,
    /// Timelock Type
    pub timelock_type: TimelockType,
    /// Voting entry rule
    pub voting_entry_rule: VotingEntryRule,
    /// Minimum slot time-distance from creation of proposal for an instruction to be placed
    pub minimum_slot_waiting_period: u64,
    /// Governance mint (optional)
    pub governance_mint: Pubkey,
    /// Program ID that is tied to this config (optional)
    pub program: Pubkey,
    /// Time limit in slots for proposal to be open to voting
    pub time_limit: u64,
    /// Optional name
    pub name: [u8; CONFIG_NAME_LENGTH],
}

impl Sealed for TimelockConfig {}
impl IsInitialized for TimelockConfig {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

/// Len of timelock config
pub const TIMELOCK_CONFIG_LEN: usize =
    1 + 1 + 1 + 1 + 1 + 8 + 32 + 32 + 8 + CONFIG_NAME_LENGTH + 300;
impl Pack for TimelockConfig {
    const LEN: usize = 1 + 1 + 1 + 1 + 1 + 8 + 32 + 32 + 8 + CONFIG_NAME_LENGTH + 300;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TIMELOCK_CONFIG_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            consensus_algorithm,
            execution_type,
            timelock_type,
            voting_entry_rule,
            minimum_slot_waiting_period,
            governance_mint,
            program,
            time_limit,
            name,
            _padding,
        ) = array_refs![input, 1, 1, 1, 1, 1, 8, 32, 32, 8, CONFIG_NAME_LENGTH, 300];
        let version = u8::from_le_bytes(*version);
        let consensus_algorithm = u8::from_le_bytes(*consensus_algorithm);
        let execution_type = u8::from_le_bytes(*execution_type);
        let timelock_type = u8::from_le_bytes(*timelock_type);
        let voting_entry_rule = u8::from_le_bytes(*voting_entry_rule);
        let minimum_slot_waiting_period = u64::from_le_bytes(*minimum_slot_waiting_period);
        let time_limit = u64::from_le_bytes(*time_limit);

        match version {
            TIMELOCK_CONFIG_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                consensus_algorithm: match consensus_algorithm {
                    0 => ConsensusAlgorithm::Majority,
                    1 => ConsensusAlgorithm::SuperMajority,
                    2 => ConsensusAlgorithm::FullConsensus,
                    _ => ConsensusAlgorithm::Majority,
                },
                execution_type: match execution_type {
                    0 => ExecutionType::Independent,
                    _ => ExecutionType::Independent,
                },
                timelock_type: match timelock_type {
                    0 => TimelockType::Governance,
                    _ => TimelockType::Governance,
                },
                voting_entry_rule: match voting_entry_rule {
                    0 => VotingEntryRule::Anytime,
                    _ => VotingEntryRule::Anytime,
                },
                minimum_slot_waiting_period,
                governance_mint: Pubkey::new_from_array(*governance_mint),
                program: Pubkey::new_from_array(*program),
                time_limit,
                name: *name,
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, TIMELOCK_CONFIG_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            consensus_algorithm,
            execution_type,
            timelock_type,
            voting_entry_rule,
            minimum_slot_waiting_period,
            governance_mint,
            program,
            time_limit,
            name,
            _padding,
        ) = mut_array_refs![output, 1, 1, 1, 1, 1, 8, 32, 32, 8, CONFIG_NAME_LENGTH, 300];
        *version = self.version.to_le_bytes();
        *consensus_algorithm = match self.consensus_algorithm {
            ConsensusAlgorithm::Majority => 0 as u8,
            ConsensusAlgorithm::SuperMajority => 1 as u8,
            ConsensusAlgorithm::FullConsensus => 2 as u8,
        }
        .to_le_bytes();
        *execution_type = match self.execution_type {
            ExecutionType::Independent => 0 as u8,
        }
        .to_le_bytes();
        *timelock_type = match self.timelock_type {
            TimelockType::Governance => 0 as u8,
        }
        .to_le_bytes();
        *voting_entry_rule = match self.voting_entry_rule {
            VotingEntryRule::Anytime => 0 as u8,
        }
        .to_le_bytes();
        *minimum_slot_waiting_period = self.minimum_slot_waiting_period.to_le_bytes();
        governance_mint.copy_from_slice(self.governance_mint.as_ref());
        program.copy_from_slice(self.program.as_ref());
        *time_limit = self.time_limit.to_le_bytes();
        name.copy_from_slice(self.name.as_ref());
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
        Ok(Self::unpack_from_slice(input)?)
    }

    fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        src.pack_into_slice(dst);
        Ok(())
    }
}
