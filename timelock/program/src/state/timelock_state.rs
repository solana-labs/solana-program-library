use crate::state::{enums, UNINITIALIZED_VERSION};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use enums::TimelockStateStatus;
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Transaction slots allowed
pub const MAX_TRANSACTIONS: usize = 5;
/// How many characters are allowed in the description
pub const DESC_SIZE: usize = 200;
/// How many characters are allowed in the name
pub const NAME_SIZE: usize = 32;
///Timelock state version
pub const TIMELOCK_STATE_VERSION: u8 = 1;
/// Timelock state
#[derive(Clone)]
pub struct TimelockState {
    /// timelock set key
    pub timelock_set: Pubkey,

    ///version
    pub version: u8,

    /// Current state of the invoked instruction account
    pub status: TimelockStateStatus,

    /// Total signatory tokens minted, for use comparing to supply remaining during draft period
    pub total_signing_tokens_minted: u64,

    /// Link to proposal
    pub desc_link: [u8; DESC_SIZE],

    /// Proposal name
    pub name: [u8; NAME_SIZE],

    /// When the timelock ended voting - this will also be when the set was defeated or began executing naturally.
    pub voting_ended_at: u64,

    /// When the timelock began voting
    pub voting_began_at: u64,

    /// when the timelock entered draft state
    pub created_at: u64,

    /// when the timelock entered completed state, also when execution ended naturally.
    pub completed_at: u64,

    /// when the timelock entered deleted state
    pub deleted_at: u64,

    /// The number of the transactions already executed
    pub number_of_executed_transactions: u8,

    /// The number of transactions included in the proposal
    pub number_of_transactions: u8,

    /// Array of pubkeys pointing at TimelockTransactions, up to 5
    pub timelock_transactions: [Pubkey; MAX_TRANSACTIONS],
}

impl Sealed for TimelockState {}
impl IsInitialized for TimelockState {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}
const TIMELOCK_STATE_LEN: usize = 32
    + 1
    + 1
    + 8
    + DESC_SIZE
    + NAME_SIZE
    + 8
    + 8
    + 8
    + 8
    + 8
    + 1
    + 1
    + (32 * MAX_TRANSACTIONS)
    + 300;
impl Pack for TimelockState {
    const LEN: usize = 32
        + 1
        + 1
        + 8
        + DESC_SIZE
        + NAME_SIZE
        + 8
        + 8
        + 8
        + 8
        + 8
        + 1
        + 1
        + (32 * MAX_TRANSACTIONS)
        + 300;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TIMELOCK_STATE_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            timelock_set,
            version,
            timelock_state_status,
            total_signing_tokens_minted,
            desc_link,
            name,
            voting_ended_at,
            voting_began_at,
            created_at,
            completed_at,
            deleted_at,
            number_of_executed_transactions,
            number_of_transactions,
            timelock_txn_1,
            timelock_txn_2,
            timelock_txn_3,
            timelock_txn_4,
            timelock_txn_5,
            _padding,
        ) = array_refs![
            input, 32, 1, 1, 8, DESC_SIZE, NAME_SIZE, 8, 8, 8, 8, 8, 1, 1, 32, 32, 32, 32, 32, 300
        ];
        let version = u8::from_le_bytes(*version);
        let total_signing_tokens_minted = u64::from_le_bytes(*total_signing_tokens_minted);
        let timelock_state_status = u8::from_le_bytes(*timelock_state_status);
        let voting_ended_at = u64::from_le_bytes(*voting_ended_at);
        let voting_began_at = u64::from_le_bytes(*voting_began_at);
        let created_at = u64::from_le_bytes(*created_at);
        let completed_at = u64::from_le_bytes(*completed_at);
        let deleted_at = u64::from_le_bytes(*deleted_at);
        let number_of_executed_transactions = u8::from_le_bytes(*number_of_executed_transactions);
        let number_of_transactions = u8::from_le_bytes(*number_of_transactions);
        match version {
            TIMELOCK_STATE_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                timelock_set: Pubkey::new_from_array(*timelock_set),
                status: match timelock_state_status {
                    0 => TimelockStateStatus::Draft,
                    1 => TimelockStateStatus::Voting,
                    2 => TimelockStateStatus::Executing,
                    3 => TimelockStateStatus::Completed,
                    4 => TimelockStateStatus::Deleted,
                    _ => TimelockStateStatus::Draft,
                },
                total_signing_tokens_minted,
                timelock_transactions: [
                    Pubkey::new_from_array(*timelock_txn_1),
                    Pubkey::new_from_array(*timelock_txn_2),
                    Pubkey::new_from_array(*timelock_txn_3),
                    Pubkey::new_from_array(*timelock_txn_4),
                    Pubkey::new_from_array(*timelock_txn_5),
                ],
                desc_link: *desc_link,
                name: *name,
                voting_ended_at,
                voting_began_at,
                created_at,
                completed_at,
                deleted_at,
                number_of_executed_transactions,
                number_of_transactions,
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, TIMELOCK_STATE_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            timelock_set,
            version,
            timelock_state_status,
            total_signing_tokens_minted,
            desc_link,
            name,
            voting_ended_at,
            voting_began_at,
            created_at,
            completed_at,
            deleted_at,
            number_of_executed_transactions,
            number_of_transactions,
            timelock_txn_1,
            timelock_txn_2,
            timelock_txn_3,
            timelock_txn_4,
            timelock_txn_5,
            _padding,
        ) = mut_array_refs![
            output, 32, 1, 1, 8, DESC_SIZE, NAME_SIZE, 8, 8, 8, 8, 8, 1, 1, 32, 32, 32, 32, 32, 300
        ];
        *version = self.version.to_le_bytes();
        timelock_set.copy_from_slice(self.timelock_set.as_ref());

        *timelock_state_status = match self.status {
            TimelockStateStatus::Draft => 0_u8,
            TimelockStateStatus::Voting => 1_u8,
            TimelockStateStatus::Executing => 2_u8,
            TimelockStateStatus::Completed => 3_u8,
            TimelockStateStatus::Deleted => 4_u8,
            TimelockStateStatus::Defeated => 5_u8,
        }
        .to_le_bytes();
        *total_signing_tokens_minted = self.total_signing_tokens_minted.to_le_bytes();
        desc_link.copy_from_slice(self.desc_link.as_ref());
        name.copy_from_slice(self.name.as_ref());
        *voting_ended_at = self.voting_ended_at.to_le_bytes();
        *voting_began_at = self.voting_began_at.to_le_bytes();
        *created_at = self.created_at.to_le_bytes();
        *completed_at = self.completed_at.to_le_bytes();
        *deleted_at = self.deleted_at.to_le_bytes();
        *number_of_executed_transactions = self.number_of_executed_transactions.to_le_bytes();
        *number_of_transactions = self.number_of_transactions.to_le_bytes();
        timelock_txn_1.copy_from_slice(self.timelock_transactions[0].as_ref());
        timelock_txn_2.copy_from_slice(self.timelock_transactions[1].as_ref());
        timelock_txn_3.copy_from_slice(self.timelock_transactions[2].as_ref());
        timelock_txn_4.copy_from_slice(self.timelock_transactions[3].as_ref());
        timelock_txn_5.copy_from_slice(self.timelock_transactions[4].as_ref());
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
