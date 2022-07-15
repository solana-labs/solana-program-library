use solana_program::pubkey::Pubkey;
use spl_governance::state::{
    enums::MintMaxVoteWeightSource,
    realm::{GoverningTokenConfigArgs, RealmConfigArgs},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SetRealmConfigArgs {
    pub realm_config_args: RealmConfigArgs,
    pub community_voter_weight_addin: Option<Pubkey>,
    pub max_community_voter_weight_addin: Option<Pubkey>,
}

impl Default for SetRealmConfigArgs {
    fn default() -> Self {
        let realm_config_args = RealmConfigArgs {
            use_council_mint: true,

            community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
            min_community_weight_to_create_governance: 10,
            community_token_config_args: GoverningTokenConfigArgs::default(),
            council_token_config_args: GoverningTokenConfigArgs::default(),
        };

        Self {
            realm_config_args,
            community_voter_weight_addin: None,
            max_community_voter_weight_addin: None,
        }
    }
}
