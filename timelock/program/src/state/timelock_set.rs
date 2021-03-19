use super::enums::TimelockStateStatus;
use super::timelock_state::{TimelockState, DESC_SIZE, NAME_SIZE};
use super::UNINITIALIZED_VERSION;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// STRUCT VERSION
pub const TIMELOCK_SET_VERSION: u8 = 1;
/// Single instance of a timelock
#[derive(Clone)]
pub struct TimelockSet {
    /// Version of the struct
    pub version: u8,

    /// Mint that creates signatory tokens of this instruction
    /// If there are outstanding signatory tokens, then cannot leave draft state. Signatories must burn tokens (ie agree
    /// to move instruction to voting state) and bring mint to net 0 tokens outstanding. Each signatory gets 1 (serves as flag)
    pub signatory_mint: Pubkey,

    /// Admin ownership mint. One token is minted, can be used to grant admin status to a new person.
    pub admin_mint: Pubkey,

    /// Mint that creates voting tokens of this instruction
    pub voting_mint: Pubkey,

    /// Mint that creates evidence of voting YES via token creation
    pub yes_voting_mint: Pubkey,

    /// Mint that creates evidence of voting NO via token creation
    pub no_voting_mint: Pubkey,

    /// Used to validate signatory tokens in a round trip transfer
    pub signatory_validation: Pubkey,

    /// Used to validate admin tokens in a round trip transfer
    pub admin_validation: Pubkey,

    /// Used to validate voting tokens in a round trip transfer
    pub voting_validation: Pubkey,

    /// Governance holding account
    pub governance_holding: Pubkey,

    /// Yes Voting dump account for exchanged vote tokens
    pub yes_voting_dump: Pubkey,

    /// No Voting dump account for exchanged vote tokens
    pub no_voting_dump: Pubkey,

    /// configuration values
    pub config: Pubkey,

    /// Timelock state
    pub state: TimelockState,
}

impl Sealed for TimelockSet {}
impl IsInitialized for TimelockSet {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const TIMELOCK_SET_LEN: usize = 540 + DESC_SIZE + NAME_SIZE;
impl Pack for TimelockSet {
    const LEN: usize = 540 + DESC_SIZE + NAME_SIZE;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TIMELOCK_SET_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            signatory_mint,
            admin_mint,
            voting_mint,
            yes_voting_mint,
            no_voting_mint,
            signatory_validation,
            admin_validation,
            voting_validation,
            governance_holding,
            yes_voting_dump,
            no_voting_dump,
            config,
            timelock_state_status,
            total_signing_tokens_minted,
            desc_link,
            name,
            voting_ended_at,
            voting_began_at,
            executions,
            used_txn_slots,
            timelock_txn_1,
            timelock_txn_2,
            timelock_txn_3,
            timelock_txn_4,
        ) = array_refs![
            input, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 1, 8, DESC_SIZE, NAME_SIZE,
            8, 8, 1, 1, 32, 32, 32, 32
        ];
        let version = u8::from_le_bytes(*version);
        let total_signing_tokens_minted = u64::from_le_bytes(*total_signing_tokens_minted);
        let timelock_state_status = u8::from_le_bytes(*timelock_state_status);
        let voting_ended_at = u64::from_le_bytes(*voting_ended_at);
        let voting_began_at = u64::from_le_bytes(*voting_began_at);
        let executions = u8::from_le_bytes(*executions);
        let used_txn_slots = u8::from_le_bytes(*used_txn_slots);
        match version {
            TIMELOCK_SET_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                signatory_mint: Pubkey::new_from_array(*signatory_mint),
                admin_mint: Pubkey::new_from_array(*admin_mint),
                voting_mint: Pubkey::new_from_array(*voting_mint),
                yes_voting_mint: Pubkey::new_from_array(*yes_voting_mint),
                no_voting_mint: Pubkey::new_from_array(*no_voting_mint),
                signatory_validation: Pubkey::new_from_array(*signatory_validation),
                admin_validation: Pubkey::new_from_array(*admin_validation),
                voting_validation: Pubkey::new_from_array(*voting_validation),
                governance_holding: Pubkey::new_from_array(*governance_holding),
                yes_voting_dump: Pubkey::new_from_array(*yes_voting_dump),
                no_voting_dump: Pubkey::new_from_array(*no_voting_dump),
                config: Pubkey::new_from_array(*config),
                state: TimelockState {
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
                    ],
                    desc_link: *desc_link,
                    name: *name,
                    voting_ended_at,
                    voting_began_at,
                    executions,
                    used_txn_slots,
                },
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, TIMELOCK_SET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            signatory_mint,
            admin_mint,
            voting_mint,
            yes_voting_mint,
            no_voting_mint,
            signatory_validation,
            admin_validation,
            voting_validation,
            governance_holding,
            yes_voting_dump,
            no_voting_dump,
            config,
            timelock_state_status,
            total_signing_tokens_minted,
            desc_link,
            name,
            voting_ended_at,
            voting_began_at,
            executions,
            used_txn_slots,
            timelock_txn_1,
            timelock_txn_2,
            timelock_txn_3,
            timelock_txn_4,
        ) = mut_array_refs![
            output, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 1, 8, DESC_SIZE, NAME_SIZE,
            8, 8, 1, 1, 32, 32, 32, 32
        ];
        *version = self.version.to_le_bytes();
        signatory_mint.copy_from_slice(self.signatory_mint.as_ref());
        admin_mint.copy_from_slice(self.admin_mint.as_ref());
        voting_mint.copy_from_slice(self.voting_mint.as_ref());
        yes_voting_mint.copy_from_slice(self.yes_voting_mint.as_ref());
        no_voting_mint.copy_from_slice(self.no_voting_mint.as_ref());
        signatory_validation.copy_from_slice(self.signatory_validation.as_ref());
        admin_validation.copy_from_slice(self.admin_validation.as_ref());
        voting_validation.copy_from_slice(self.voting_validation.as_ref());
        governance_holding.copy_from_slice(self.governance_holding.as_ref());
        yes_voting_dump.copy_from_slice(self.yes_voting_dump.as_ref());
        no_voting_dump.copy_from_slice(self.no_voting_dump.as_ref());
        config.copy_from_slice(self.config.as_ref());
        *timelock_state_status = match self.state.status {
            TimelockStateStatus::Draft => 0 as u8,
            TimelockStateStatus::Voting => 1 as u8,
            TimelockStateStatus::Executing => 2 as u8,
            TimelockStateStatus::Completed => 3 as u8,
            TimelockStateStatus::Deleted => 4 as u8,
            TimelockStateStatus::Defeated => 5 as u8,
        }
        .to_le_bytes();
        *total_signing_tokens_minted = self.state.total_signing_tokens_minted.to_le_bytes();
        desc_link.copy_from_slice(self.state.desc_link.as_ref());
        name.copy_from_slice(self.state.name.as_ref());
        *voting_ended_at = self.state.voting_ended_at.to_le_bytes();
        *voting_began_at = self.state.voting_began_at.to_le_bytes();
        *executions = self.state.executions.to_le_bytes();
        *used_txn_slots = self.state.used_txn_slots.to_le_bytes();
        timelock_txn_1.copy_from_slice(self.state.timelock_transactions[0].as_ref());
        timelock_txn_2.copy_from_slice(self.state.timelock_transactions[1].as_ref());
        timelock_txn_3.copy_from_slice(self.state.timelock_transactions[2].as_ref());
        timelock_txn_4.copy_from_slice(self.state.timelock_transactions[3].as_ref());
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
