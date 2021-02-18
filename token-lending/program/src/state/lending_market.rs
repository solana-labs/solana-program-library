use super::*;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Lending market state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LendingMarket {
    /// Version of lending market
    pub version: u8,
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Quote currency token mint
    pub quote_token_mint: Pubkey,
    /// Token program id
    pub token_program_id: Pubkey,
}

impl Sealed for LendingMarket {}
impl IsInitialized for LendingMarket {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const LENDING_MARKET_LEN: usize = 160;
impl Pack for LendingMarket {
    const LEN: usize = 160;

    /// Unpacks a byte buffer into a [LendingMarketInfo](struct.LendingMarketInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, bump_seed, owner, quote_token_mint, token_program_id, _padding) =
            array_refs![input, 1, 1, 32, 32, 32, 62];
        let version = u8::from_le_bytes(*version);
        if version > PROGRAM_VERSION {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            version,
            bump_seed: u8::from_le_bytes(*bump_seed),
            owner: Pubkey::new_from_array(*owner),
            quote_token_mint: Pubkey::new_from_array(*quote_token_mint),
            token_program_id: Pubkey::new_from_array(*token_program_id),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, bump_seed, owner, quote_token_mint, token_program_id, _padding) =
            mut_array_refs![output, 1, 1, 32, 32, 32, 62];
        *version = self.version.to_le_bytes();
        *bump_seed = self.bump_seed.to_le_bytes();
        owner.copy_from_slice(self.owner.as_ref());
        quote_token_mint.copy_from_slice(self.quote_token_mint.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
    }
}
