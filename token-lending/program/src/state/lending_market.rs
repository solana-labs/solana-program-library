use super::*;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::{Pubkey, PUBKEY_BYTES},
};

/// Lending market state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LendingMarket {
    /// Version of lending market
    pub version: u8,
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Token program id
    pub token_program_id: Pubkey,
    /// Quote currency token mint
    pub quote_token_mint: Pubkey,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
}

/// Initialize a lending market
pub struct InitLendingMarketParams {
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Token program id
    pub token_program_id: Pubkey,
    /// Quote currency token mint
    pub quote_token_mint: Pubkey,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
}

impl LendingMarket {
    /// Create a new lending market
    pub fn new(params: InitLendingMarketParams) -> Self {
        let mut lending_market = Self::default();
        Self::init(&mut lending_market, params);
        lending_market
    }

    /// Initialize a lending market
    pub fn init(&mut self, params: InitLendingMarketParams) {
        self.version = PROGRAM_VERSION;
        self.bump_seed = params.bump_seed;
        self.token_program_id = params.token_program_id;
        self.quote_token_mint = params.quote_token_mint;
        self.owner = params.owner;
    }
}

impl Sealed for LendingMarket {}
impl IsInitialized for LendingMarket {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

// @TODO: Adjust padding, but what's a reasonable number?
//        Or should there be no padding to save space, but we need account resizing implemented?
const LENDING_MARKET_LEN: usize = 226; // 1 + 1 + 32 + 32 + 32 + 128
impl Pack for LendingMarket {
    const LEN: usize = LENDING_MARKET_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, bump_seed, token_program_id, quote_token_mint, owner, _padding) =
            mut_array_refs![output, 1, 1, PUBKEY_BYTES, PUBKEY_BYTES, PUBKEY_BYTES, 128];
        *version = self.version.to_le_bytes();
        *bump_seed = self.bump_seed.to_le_bytes();
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        quote_token_mint.copy_from_slice(self.quote_token_mint.as_ref());
        owner.copy_from_slice(self.owner.as_ref());
    }

    /// Unpacks a byte buffer into a [LendingMarketInfo](struct.LendingMarketInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, bump_seed, token_program_id, quote_token_mint, owner, _padding) =
            array_refs![input, 1, 1, PUBKEY_BYTES, PUBKEY_BYTES, PUBKEY_BYTES, 128];
        let version = u8::from_le_bytes(*version);
        if version > PROGRAM_VERSION {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            version,
            bump_seed: u8::from_le_bytes(*bump_seed),
            token_program_id: Pubkey::new_from_array(*token_program_id),
            quote_token_mint: Pubkey::new_from_array(*quote_token_mint),
            owner: Pubkey::new_from_array(*owner),
        })
    }
}
