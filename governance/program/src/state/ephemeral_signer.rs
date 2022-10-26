//! Ephemeral signer
use solana_program::pubkey::Pubkey;
use std::convert::TryFrom;

use super::proposal_transaction::{ProposalTransactionV2, SignerType};

/// Returns ephemeral signer seeds number `account_seq_number_le_bytes` for the 
/// proposal transaction at address `proposal_transaction_pubkey`
pub fn get_ephemeral_signer_seeds<'a>(
    proposal_transaction_pubkey: &'a Pubkey,
    account_seq_number_le_bytes: &'a [u8; 2],
) -> [&'a [u8]; 3] {
    [
        b"ephemeral-signer",
        proposal_transaction_pubkey.as_ref(),
        account_seq_number_le_bytes,
    ]
}

/// Returns ephemeral signer address number `account_seq_number_le_bytes` for the 
/// proposal transaction at address `proposal_transaction_pubkey`
pub fn get_ephemeral_signer_address(
    program_id: &Pubkey,
    proposal_transaction_pubkey: &Pubkey,
    account_seq_number_le_bytes: &[u8; 2],
) -> Pubkey {
    let seeds =
        &get_ephemeral_signer_seeds(proposal_transaction_pubkey, account_seq_number_le_bytes);
    Pubkey::find_program_address(seeds, program_id).0
}

/// This struct stores all the data needed for using the ephemeral seeds
/// when doing a CPI.
/// Invoking another program requires the seeds to be passed as `&[&[&[u8]]]`.
/// But all these references need to point to variables owned by the 
/// block that calls `invoke`. This struct stores those variables.
pub struct EphemeralSeedGenerator<'a> {
    /// The account sequence numbers serialized with little-endianness (u16 = 2 bytes)
    pub account_seq_numbers: Vec<[u8; 2]>,
    /// The bump seeds 
    pub bump_seeds: Vec<[u8; 1]>,
    /// References to the signer seeds and the bump `[seeds[0], seeds[1], seeds[2], bump]`
    pub signers_seeds_with_bump: Vec<[&'a [u8]; 4]>,
}

impl<'a> Default for EphemeralSeedGenerator<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> EphemeralSeedGenerator<'a> {
    /// Initialize an empty `EphemeralSeedGenerator`
    pub fn new() -> Self {
        EphemeralSeedGenerator {
            account_seq_numbers: vec![],
            bump_seeds: vec![],
            signers_seeds_with_bump: vec![],
        }
    }

    /// Returns the signer seeds for all the ephemeral accounts involved in executing `proposal_transaction_data`
    pub fn get_proposal_transaction_ephemeral_signer_seeds(
        &'a mut self,
        program_id: &Pubkey,
        proposal_transaction_pubkey: &'a Pubkey,
        proposal_transaction_data: &ProposalTransactionV2,
    ) -> Vec<&[&'a [u8]]> {
        let number_of_ephemeral_accounts: usize = proposal_transaction_data
            .instructions
            .iter()
            .flat_map(|ix| &ix.accounts)
            .filter(|acc| acc.is_signer == SignerType::Ephemeral)
            .count();

        let mut signer_seeds = vec![];

        self.account_seq_numbers = (0..number_of_ephemeral_accounts)
            .map(|x| u16::try_from(x).unwrap().to_le_bytes())
            .collect();

        for account_seq_number_le_bytes in self.account_seq_numbers.iter() {
            let seeds: [&[u8]; 3] = get_ephemeral_signer_seeds(
                proposal_transaction_pubkey,
                account_seq_number_le_bytes,
            );
            let (_, bump) = Pubkey::find_program_address(&seeds, program_id);
            self.bump_seeds.push([bump]);
            signer_seeds.push(seeds);
        }

        self.signers_seeds_with_bump = signer_seeds
            .iter()
            .zip(self.bump_seeds.iter())
            .map(|(seeds, bump)| [seeds[0], seeds[1], seeds[2], bump])
            .collect();
        self.signers_seeds_with_bump
            .iter()
            .map(|x| &x[..])
            .collect()
    }
}
