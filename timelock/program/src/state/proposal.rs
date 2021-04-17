use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use crate::state::enums::GovernanceAccountType;

/// Single instance of a Governance proposal
#[derive(Clone)]
pub struct Proposal {
    /// Account type
    pub account_type: GovernanceAccountType,

    /// configuration values
    pub config: Pubkey,

    /// Token program used
    pub token_program_id: Pubkey,

    /// state values
    pub state: Pubkey,

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

    /// Source token holding account
    pub source_holding: Pubkey,

    /// Source mint - either governance or council mint from config
    pub source_mint: Pubkey,

    /// Yes Voting dump account for exchanged vote tokens
    pub yes_voting_dump: Pubkey,

    /// No Voting dump account for exchanged vote tokens
    pub no_voting_dump: Pubkey,
}

impl Sealed for Proposal {}
impl IsInitialized for Proposal {
    fn is_initialized(&self) -> bool {
        self.account_type != GovernanceAccountType::Uninitialized
    }
}

const PROPOSAL_LEN: usize = 1 + 32 * 15 + 300;
impl Pack for Proposal {
    const LEN: usize = 1 + 32 * 15 + 300;
    /// Unpacks a byte buffer into a Proposal account data
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, PROPOSAL_LEN];
        // TODO think up better way than txn_* usage here - new to rust
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            account_type_value,
            config,
            token_program_id,
            state,
            signatory_mint,
            admin_mint,
            voting_mint,
            yes_voting_mint,
            no_voting_mint,
            source_mint,
            signatory_validation,
            admin_validation,
            voting_validation,
            source_holding,
            yes_voting_dump,
            no_voting_dump,
            _padding,
        ) = array_refs![input, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 300];
        let account_type = u8::from_le_bytes(*account_type_value);

        let account_type = match account_type {
            0 => GovernanceAccountType::Uninitialized,
            2 => GovernanceAccountType::Proposal,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Self {
            account_type,
            config: Pubkey::new_from_array(*config),
            token_program_id: Pubkey::new_from_array(*token_program_id),
            state: Pubkey::new_from_array(*state),
            signatory_mint: Pubkey::new_from_array(*signatory_mint),
            admin_mint: Pubkey::new_from_array(*admin_mint),
            voting_mint: Pubkey::new_from_array(*voting_mint),
            yes_voting_mint: Pubkey::new_from_array(*yes_voting_mint),
            no_voting_mint: Pubkey::new_from_array(*no_voting_mint),
            source_mint: Pubkey::new_from_array(*source_mint),
            signatory_validation: Pubkey::new_from_array(*signatory_validation),
            admin_validation: Pubkey::new_from_array(*admin_validation),
            voting_validation: Pubkey::new_from_array(*voting_validation),
            source_holding: Pubkey::new_from_array(*source_holding),
            yes_voting_dump: Pubkey::new_from_array(*yes_voting_dump),
            no_voting_dump: Pubkey::new_from_array(*no_voting_dump),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, PROPOSAL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            account_type_value,
            config,
            token_program_id,
            state,
            signatory_mint,
            admin_mint,
            voting_mint,
            yes_voting_mint,
            no_voting_mint,
            source_mint,
            signatory_validation,
            admin_validation,
            voting_validation,
            source_holding,
            yes_voting_dump,
            no_voting_dump,
            _padding,
        ) = mut_array_refs![
            output, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 300
        ];

        *account_type_value = match self.account_type {
            GovernanceAccountType::Uninitialized => 0_u8,
            GovernanceAccountType::Proposal => 2_u8,
            _ => panic!("Account type was invalid"),
        }
        .to_le_bytes();

        config.copy_from_slice(self.config.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        state.copy_from_slice(self.state.as_ref());
        signatory_mint.copy_from_slice(self.signatory_mint.as_ref());
        admin_mint.copy_from_slice(self.admin_mint.as_ref());
        voting_mint.copy_from_slice(self.voting_mint.as_ref());
        yes_voting_mint.copy_from_slice(self.yes_voting_mint.as_ref());
        no_voting_mint.copy_from_slice(self.no_voting_mint.as_ref());
        source_mint.copy_from_slice(self.source_mint.as_ref());
        signatory_validation.copy_from_slice(self.signatory_validation.as_ref());
        admin_validation.copy_from_slice(self.admin_validation.as_ref());
        voting_validation.copy_from_slice(self.voting_validation.as_ref());
        source_holding.copy_from_slice(self.source_holding.as_ref());
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
