//! Ephemeral signer
use solana_program::pubkey::Pubkey;

/// TO DO DOCUMENTATION
pub fn get_ephemeral_signer_seeds<'a>(proposal_transaction_pubkey: &'a Pubkey, account_seq_number_le_bytes : &'a [u8; 2]) -> [&'a [u8]; 3] {
    [b"ephemeral-signer", proposal_transaction_pubkey.as_ref(), account_seq_number_le_bytes]
}

/// Returns ProposalExtraAccount PDA address
pub fn get_ephemeral_signer_address_and_seeds<'a>(program_id: &Pubkey, proposal_transaction_pubkey: &'a Pubkey, account_seq_number_le_bytes : &'a [u8; 2]) -> (Pubkey, u8, Vec<&'a [u8]>)  {
    let seeds = &get_ephemeral_signer_seeds(proposal_transaction_pubkey, account_seq_number_le_bytes);
    let (address, bump) = Pubkey::find_program_address(seeds, program_id);
    let seeds_vec = seeds.to_vec();
    return (address, bump, seeds_vec)
}

/// Returns ProposalExtraAccount PDA address
pub fn get_ephemeral_signer_address(program_id: &Pubkey, proposal_transaction_pubkey: &Pubkey, account_seq_number_le_bytes : &[u8; 2]) -> Pubkey  {
    let seeds = &get_ephemeral_signer_seeds(proposal_transaction_pubkey, &account_seq_number_le_bytes);
    Pubkey::find_program_address(seeds, program_id).0
}
