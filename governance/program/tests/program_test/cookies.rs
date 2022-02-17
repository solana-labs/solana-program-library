use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::signature::Keypair;
use spl_governance::state::{
    governance::GovernanceV2, native_treasury::NativeTreasury, program_metadata::ProgramMetadata,
    proposal::ProposalV2, proposal_transaction::ProposalTransactionV2, realm::RealmV2,
    realm_config::RealmConfigAccount, signatory_record::SignatoryRecordV2,
    token_owner_record::TokenOwnerRecordV2, vote_record::VoteRecordV2,
};

use spl_governance_addin_api::{
    max_voter_weight::MaxVoterWeightRecord, voter_weight::VoterWeightRecord,
};
use spl_governance_test_sdk::tools::clone_keypair;

pub trait AccountCookie {
    fn get_address(&self) -> Pubkey;
}

#[derive(Debug)]
pub struct RealmCookie {
    pub address: Pubkey,

    pub account: RealmV2,

    pub community_mint_authority: Keypair,

    pub community_token_holding_account: Pubkey,

    pub council_mint_authority: Option<Keypair>,

    pub council_token_holding_account: Option<Pubkey>,

    pub realm_authority: Option<Keypair>,

    pub realm_config: Option<RealmConfigCookie>,
}

#[derive(Debug)]
pub struct RealmConfigCookie {
    pub address: Pubkey,
    pub account: RealmConfigAccount,
}

#[derive(Debug)]
pub struct TokenOwnerRecordCookie {
    pub address: Pubkey,

    pub account: TokenOwnerRecordV2,

    pub token_source: Pubkey,

    pub token_source_amount: u64,

    pub token_owner: Keypair,

    pub governance_authority: Option<Keypair>,

    pub governance_delegate: Keypair,

    pub voter_weight_record: Option<VoterWeightRecordCookie>,

    // This doesn't belong to TokenOwnerRecord and I put it here for simplicity for now
    pub max_voter_weight_record: Option<MaxVoterWeightRecordCookie>,
}

impl TokenOwnerRecordCookie {
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

impl AccountCookie for GovernedProgramCookie {
    fn get_address(&self) -> Pubkey {
        self.address
    }
}

#[derive(Debug)]
pub struct GovernedMintCookie {
    pub address: Pubkey,
    pub mint_authority: Keypair,
    pub transfer_mint_authority: bool,
}

impl AccountCookie for GovernedMintCookie {
    fn get_address(&self) -> Pubkey {
        self.address
    }
}

#[derive(Debug)]
pub struct GovernedTokenCookie {
    pub address: Pubkey,
    pub token_owner: Keypair,
    pub transfer_token_owner: bool,
    pub token_mint: Pubkey,
}

impl AccountCookie for GovernedTokenCookie {
    fn get_address(&self) -> Pubkey {
        self.address
    }
}

#[derive(Debug)]
pub struct GovernedAccountCookie {
    pub address: Pubkey,
}

impl AccountCookie for GovernedAccountCookie {
    fn get_address(&self) -> Pubkey {
        self.address
    }
}

#[derive(Debug)]
pub struct GovernanceCookie {
    pub address: Pubkey,
    pub account: GovernanceV2,
    pub next_proposal_index: u32,
}

#[derive(Debug)]
pub struct ProposalCookie {
    pub address: Pubkey,
    pub account: ProposalV2,

    pub realm: Pubkey,
    pub proposal_owner: Pubkey,
}

#[derive(Debug)]
pub struct SignatoryRecordCookie {
    pub address: Pubkey,
    pub account: SignatoryRecordV2,
    pub signatory: Keypair,
}

#[derive(Debug)]
pub struct VoteRecordCookie {
    pub address: Pubkey,
    pub account: VoteRecordV2,
}

#[derive(Debug)]
pub struct ProposalTransactionCookie {
    pub address: Pubkey,
    pub account: ProposalTransactionV2,
    pub instruction: Instruction,
}

#[derive(Debug, Clone)]
pub struct VoterWeightRecordCookie {
    pub address: Pubkey,
    pub account: VoterWeightRecord,
}

#[derive(Debug, Clone)]
pub struct MaxVoterWeightRecordCookie {
    pub address: Pubkey,
    pub account: MaxVoterWeightRecord,
}

#[derive(Debug, Clone)]
pub struct ProgramMetadataCookie {
    pub address: Pubkey,
    pub account: ProgramMetadata,
}

#[derive(Debug, Clone)]
pub struct NativeTreasuryCookie {
    pub address: Pubkey,
    pub account: NativeTreasury,
}
