use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use spl_governance::state::{governance::Governance, realm::Realm, voter_record::VoterRecord};

#[derive(Debug)]
pub struct RealmCookie {
    pub address: Pubkey,

    pub account: Realm,

    pub community_mint_authority: Keypair,

    pub community_token_holding_account: Pubkey,

    pub council_mint_authority: Option<Keypair>,

    pub council_token_holding_account: Option<Pubkey>,
}

#[derive(Debug)]
pub struct VoterRecordCookie {
    pub address: Pubkey,

    pub account: VoterRecord,

    pub token_source: Pubkey,

    pub token_source_amount: u64,

    pub token_owner: Keypair,

    pub vote_authority: Keypair,
}

#[derive(Debug)]
pub struct GovernedProgramCookie {
    pub address: Pubkey,
    pub upgrade_authority: Keypair,
    pub data_address: Pubkey,
    pub transfer_upgrade_authority: bool,
}

#[derive(Debug)]
pub struct GovernedAccountCookie {
    pub address: Pubkey,
}

#[derive(Debug)]
pub struct GovernanceCookie {
    pub address: Pubkey,
    pub account: Governance,
}
