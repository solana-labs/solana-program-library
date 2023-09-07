//! The beahvior of `ArgMatches::is_present` and `ArgMatches::value_of` differ in `clap_v2` and
//! `clap_v3`. This submodule contains adaptation of these functions to preserve their v2 behavior in
//! v3.

use clap::ArgMatches;
use solana_clap_v3_utils::keypair::pubkey_from_path;
use solana_remote_wallet::remote_wallet::RemoteWalletManager;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

/// Adaptation of `ArgMatches::is_present` that has the same behavior in `clap_v3` as in `clap_v2`.
///
/// The function `ArgMatches::is_present` behaves differently in clap v2 and clap v3:
///   - In v2, `index_of` returns `true` on success and `false` on all other cases.
///   - In v3, `index_of` returns `true` on success, `false` if the queried argument id
///     is valid but the argument was not provided, and panics if the input argument id does not
///     exist.
/// This adaptation behaves as in `clap_v2`.
pub(crate) fn is_present(matches: &ArgMatches, name: &str) -> bool {
    matches.try_contains_id(name).unwrap_or(false)
}

/// Adaptation of `ArgMatches::value_of` that has the same behavior in `clap_v3` as in `clap_v2`.
///
/// The function `ArgMatches::value_of` behaves differently in clap v2 and clap v3:
///   - In v2, `value_of` returns `Some(...)` on success and `None` on all other cases.
///   - In v3, `value_of` returns `Some(...)` on success, `None` if the queried argument id
///     is valid but the argument was not provided, and panics if the input argument id does not
///     exist.
/// This adaptation behaves as in `clap_v2`.
pub(crate) fn value_of<T>(matches: &ArgMatches, name: &str) -> Option<T>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    let maybe_value = matches.try_get_one::<String>(name).unwrap_or(None);
    if let Some(value) = maybe_value {
        value.parse::<T>().ok()
    } else {
        None
    }
}

/// Adaptation of `solana_clap_v3_utils::input_parsers::pubkey_of_signer` that has the same
/// behavior as `solana_clap_utils::input_parsers::pubkey_of_signer`.
pub(crate) fn pubkey_of_signer(
    matches: &ArgMatches,
    name: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Result<Option<Pubkey>, Box<dyn std::error::Error>> {
    if let Some(location) = matches.try_get_one::<String>(name).ok().flatten() {
        Ok(Some(pubkey_from_path(
            matches,
            location,
            name,
            wallet_manager,
        )?))
    } else {
        Ok(None)
    }
}
