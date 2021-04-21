use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

pub struct PerpetualSwap {
    pub is_initialized: bool,
    pub nonce: u8,
    pub token_program_id: Pubkey,
    pub long_margin_pubkey: Pubkey,
    pub long_account_pubkey: Pubkey,
    pub short_margin_pubkey: Pubkey,
    pub short_account_pubkey: Pubkey,
    pub reference_time: u128,
    pub index_price: f64,
    pub mark_price: f64,
    pub minimum_margin: u64,
    pub liquidation_threshold: f64,
    pub funding_rate: f64,
}

impl Sealed for PerpetualSwap {}

impl IsInitialized for PerpetualSwap {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for PerpetualSwap {
    const LEN: usize = 218;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, PerpetualSwap::LEN];
        let (
            is_initialized,
            nonce,
            token_program_id,
            long_margin_pubkey,
            long_account_pubkey,
            short_margin_pubkey,
            short_account_pubkey,
            reference_time,
            index_price,
            mark_price,
            minimum_margin,
            liquidation_threshold,
            funding_rate,
        ) = array_refs![src, 1, 1, 32, 32, 32, 32, 32, 16, 8, 8, 8, 8, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(PerpetualSwap {
            is_initialized,
            nonce: u8::from_le_bytes(*nonce),
            token_program_id: Pubkey::new_from_array(*token_program_id),
            long_margin_pubkey: Pubkey::new_from_array(*long_margin_pubkey),
            long_account_pubkey: Pubkey::new_from_array(*long_account_pubkey),
            short_margin_pubkey: Pubkey::new_from_array(*short_margin_pubkey),
            short_account_pubkey: Pubkey::new_from_array(*short_account_pubkey),
            reference_time: u128::from_le_bytes(*reference_time),
            index_price: f64::from_le_bytes(*index_price),
            mark_price: f64::from_le_bytes(*mark_price),
            minimum_margin: u64::from_le_bytes(*minimum_margin),
            liquidation_threshold: f64::from_le_bytes(*liquidation_threshold),
            funding_rate: f64::from_le_bytes(*funding_rate),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, PerpetualSwap::LEN];
        let (
            is_initialized_dst,
            nonce_dst,
            token_program_id_dst,
            long_margin_pubkey_dst,
            long_account_pubkey_dst,
            short_margin_pubkey_dst,
            short_account_pubkey_dst,
            reference_time_dst,
            index_price_dst,
            mark_price_dst,
            minimum_margin_dst,
            liquidation_threshold_dst,
            funding_rate_dst,
        ) = mut_array_refs![dst, 1, 1, 32, 32, 32, 32, 32, 16, 8, 8, 8, 8, 8];

        let PerpetualSwap {
            is_initialized,
            nonce,
            token_program_id,
            long_margin_pubkey,
            long_account_pubkey,
            short_margin_pubkey,
            short_account_pubkey,
            reference_time,
            index_price,
            mark_price,
            minimum_margin,
            liquidation_threshold,
            funding_rate,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        *nonce_dst = nonce.to_le_bytes();
        token_program_id_dst.copy_from_slice(token_program_id.as_ref());
        long_margin_pubkey_dst.copy_from_slice(long_margin_pubkey.as_ref());
        long_account_pubkey_dst.copy_from_slice(long_account_pubkey.as_ref());
        short_margin_pubkey_dst.copy_from_slice(short_margin_pubkey.as_ref());
        short_account_pubkey_dst.copy_from_slice(short_account_pubkey.as_ref());
        *reference_time_dst = reference_time.to_le_bytes();
        *index_price_dst = index_price.to_le_bytes();
        *mark_price_dst = mark_price.to_le_bytes();
        *minimum_margin_dst = minimum_margin.to_le_bytes();
        *liquidation_threshold_dst = liquidation_threshold.to_le_bytes();
        *funding_rate_dst = funding_rate.to_le_bytes();
    }

}