use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use spl_governance_chat::state::Message;

#[derive(Debug)]
pub struct MessageCookie {
    pub address: Pubkey,
    pub account: Message,
}

#[derive(Debug)]
pub struct ProposalCookie {
    pub address: Pubkey,
    pub realm_address: Pubkey,
    pub governance_address: Pubkey,
    pub token_owner_record_address: Pubkey,
    pub token_owner: Keypair,
}
