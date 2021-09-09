use solana_program::pubkey::Pubkey;
use spl_governance_chat::state::Message;

#[derive(Debug)]
pub struct MessageCookie {
    pub address: Pubkey,
    pub account: Message,
}
