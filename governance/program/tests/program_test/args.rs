use solana_program::pubkey::Pubkey;
use spl_governance::state::realm::RealmConfigArgs;

#[derive(Clone, Debug, PartialEq)]
pub struct SetRealmConfigArgs {
    pub realm_config_args: RealmConfigArgs,
    pub community_voter_weight_addin: Option<Pubkey>,
    pub max_community_voter_weight_addin: Option<Pubkey>,
}
