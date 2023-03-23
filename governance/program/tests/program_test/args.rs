use spl_governance::state::{
    enums::MintMaxVoterWeightSource, realm::GoverningTokenConfigAccountArgs,
};

#[derive(Clone, Debug, PartialEq)]
pub struct RealmSetupArgs {
    pub use_council_mint: bool,
    pub min_community_weight_to_create_governance: u64,
    pub community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
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
            community_mint_max_voter_weight_source: MintMaxVoterWeightSource::FULL_SUPPLY_FRACTION,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginSetupArgs {
    pub use_community_voter_weight_addin: bool,
    pub use_max_community_voter_weight_addin: bool,
    pub use_council_voter_weight_addin: bool,
    pub use_max_council_voter_weight_addin: bool,
}

impl PluginSetupArgs {
    #[allow(dead_code)]
    pub const COMMUNITY_VOTER_WEIGHT: PluginSetupArgs = PluginSetupArgs {
        use_community_voter_weight_addin: true,
        use_max_community_voter_weight_addin: false,
        use_council_voter_weight_addin: false,
        use_max_council_voter_weight_addin: false,
    };
    #[allow(dead_code)]
    pub const COMMUNITY_MAX_VOTER_WEIGHT: PluginSetupArgs = PluginSetupArgs {
        use_community_voter_weight_addin: false,
        use_max_community_voter_weight_addin: true,
        use_council_voter_weight_addin: false,
        use_max_council_voter_weight_addin: false,
    };
    #[allow(dead_code)]
    pub const COUNCIL_VOTER_WEIGHT: PluginSetupArgs = PluginSetupArgs {
        use_community_voter_weight_addin: false,
        use_max_community_voter_weight_addin: false,
        use_council_voter_weight_addin: true,
        use_max_council_voter_weight_addin: false,
    };
    #[allow(dead_code)]
    pub const COUNCIL_MAX_VOTER_WEIGHT: PluginSetupArgs = PluginSetupArgs {
        use_community_voter_weight_addin: false,
        use_max_community_voter_weight_addin: false,
        use_council_voter_weight_addin: false,
        use_max_council_voter_weight_addin: true,
    };
    #[allow(dead_code)]
    pub const ALL: PluginSetupArgs = PluginSetupArgs {
        use_community_voter_weight_addin: true,
        use_max_community_voter_weight_addin: true,
        use_council_voter_weight_addin: true,
        use_max_council_voter_weight_addin: true,
    };
}
