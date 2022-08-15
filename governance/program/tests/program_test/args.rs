use spl_governance::state::{
    enums::MintMaxVoteWeightSource, realm::GoverningTokenConfigAccountArgs,
};

#[derive(Clone, Debug, PartialEq)]
pub struct RealmSetupArgs {
    pub use_council_mint: bool,
    pub min_community_weight_to_create_governance: u64,
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,
    pub community_token_config_args: GoverningTokenConfigAccountArgs,
    pub council_token_config_args: GoverningTokenConfigAccountArgs,
}

impl Default for RealmSetupArgs {
    fn default() -> Self {
        Self {
            use_council_mint: true,
            community_token_config_args: GoverningTokenConfigAccountArgs::default(),
            council_token_config_args: GoverningTokenConfigAccountArgs::default(),
            min_community_weight_to_create_governance: 10,
            community_mint_max_vote_weight_source: MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
        }
    }
}
