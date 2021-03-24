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
    /// configuration values
    pub config: Pubkey,

    /// state values
    pub state: Pubkey,

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
}

impl Sealed for TimelockSet {}
impl IsInitialized for TimelockSet {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const TIMELOCK_SET_LEN: usize = 1 + 32 * 13;
impl Pack for TimelockSet {
    const LEN: usize = 1 + 32 * 13;
    /// Unpacks a byte buffer into a [TimelockProgram](struct.TimelockProgram.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TIMELOCK_SET_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            config,
            state,
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
        ) = array_refs![input, 32, 32, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32];
        let version = u8::from_le_bytes(*version);
        match version {
            TIMELOCK_SET_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                config: Pubkey::new_from_array(*config),
                state: Pubkey::new_from_array(*state),
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
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, TIMELOCK_SET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            config,
            state,
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
        ) = mut_array_refs![output, 32, 32, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32];
        config.copy_from_slice(self.config.as_ref());
        state.copy_from_slice(self.state.as_ref());
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
