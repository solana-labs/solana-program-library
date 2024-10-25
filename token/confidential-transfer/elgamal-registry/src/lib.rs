mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;

use solana_program::pubkey::Pubkey;

/// Seed for the ElGamal registry program-derived address
pub const REGISTRY_ADDRESS_SEED: &[u8] = b"elgamal-registry";

/// Derives the ElGamal registry account address and seed for the given wallet
/// address
pub fn get_elgamal_registry_address_and_bump_seed(
    wallet_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[REGISTRY_ADDRESS_SEED, wallet_address.as_ref()],
        program_id,
    )
}

/// Derives the ElGamal registry account address for the given wallet address
pub fn get_elgamal_registry_address(wallet_address: &Pubkey, program_id: &Pubkey) -> Pubkey {
    get_elgamal_registry_address_and_bump_seed(wallet_address, program_id).0
}

solana_program::declare_id!("regVYJW7tcT8zipN5YiBvHsvR5jXW1uLFxaHSbugABg");
