//! Defines all program-derived addresses used by the program

use solana_program::pubkey::Pubkey;

macro_rules! impl_program_address {
    ($address:ident, $seed:ident) => {
        /// Defines a program-derived address
        pub struct $address;
        impl $address {
            /// Generates the program address
            pub fn find(
                program_id: &Pubkey,
                validator: &Pubkey,
                stake_pool_program_id: &Pubkey,
            ) -> (Pubkey, u8) {
                Pubkey::find_program_address(
                    &Self::collect_seeds(validator, stake_pool_program_id),
                    program_id,
                )
            }

            /// Generates the seeds for checking
            pub(crate) fn collect_seeds<'a>(
                validator: &'a Pubkey,
                stake_pool_program_id: &'a Pubkey,
            ) -> [&'a [u8]; 3] {
                [$seed, validator.as_ref(), stake_pool_program_id.as_ref()]
            }

            /// Generates the seeds for signing
            pub(crate) fn collect_signer_seeds<'a>(
                validator: &'a Pubkey,
                stake_pool_program_id: &'a Pubkey,
                bump_seed: &'a [u8],
            ) -> [&'a [u8]; 4] {
                [
                    $seed,
                    validator.as_ref(),
                    stake_pool_program_id.as_ref(),
                    bump_seed,
                ]
            }
        }
    };
}

/// Seed for stake pool manager and staker
const MANAGER_SEED: &[u8] = b"manager";
impl_program_address!(ManagerAddress, MANAGER_SEED);

/// Seed for stake pool address
const STAKE_POOL_SEED: &[u8] = b"stake-pool";
impl_program_address!(StakePoolAddress, STAKE_POOL_SEED);

/// Seed for validator list address
const VALIDATOR_LIST_SEED: &[u8] = b"validator-list";
impl_program_address!(ValidatorListAddress, VALIDATOR_LIST_SEED);

/// Seed for stake pool mint
const MINT_SEED: &[u8] = b"mint";
impl_program_address!(MintAddress, MINT_SEED);

/// Seed for stake pool reserve
const RESERVE_SEED: &[u8] = b"reserve";
impl_program_address!(ReserveAddress, RESERVE_SEED);
