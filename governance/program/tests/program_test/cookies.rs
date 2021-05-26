use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use spl_governance::state::{
    governance::Governance, realm::Realm, token_owner_record::TokenOwnerRecord,
};
use spl_governance::state::{proposal::Proposal, signatory_record::SignatoryRecord};

use super::tools::clone_keypair;

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
pub struct TokeOwnerRecordCookie {
    pub address: Pubkey,

    pub account: TokenOwnerRecord,

    pub token_source: Pubkey,

    pub token_source_amount: u64,

    pub token_owner: Keypair,

    pub governance_authority: Option<Keypair>,

    pub governance_delegate: Keypair,

    pub governing_token_mint: Pubkey,
}

impl TokeOwnerRecordCookie {
    pub fn get_governance_authority(&self) -> &Keypair {
        self.governance_authority
            .as_ref()
            .unwrap_or(&self.token_owner)
    }

    #[allow(dead_code)]
    pub fn clone_governance_delegate(&self) -> Keypair {
        clone_keypair(&self.governance_delegate)
    }
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
    pub next_proposal_index: u16,
}

#[derive(Debug)]
pub struct ProposalCookie {
    pub address: Pubkey,
    pub account: Proposal,

    pub proposal_owner: Pubkey,
}

#[derive(Debug)]
pub struct SignatoryRecordCookie {
    pub address: Pubkey,
    pub account: SignatoryRecord,
    pub signatory: Keypair,
}
