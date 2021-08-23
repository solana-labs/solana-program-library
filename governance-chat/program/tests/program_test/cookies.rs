use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use spl_governance_chat::state::ChatMessage;

#[derive(Debug)]
pub struct ChatMessageCookie {
    pub address: Pubkey,
    pub account: ChatMessage,
}

#[derive(Debug)]
pub struct ProposalCookie {
    pub address: Pubkey,
    pub realm_address: Pubkey,
    pub governance_address: Pubkey,
    pub token_owner_record_address: Pubkey,
    pub token_owner: Keypair,
}
