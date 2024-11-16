//! Temporary ElGamal keypair argument parser.
//!
//! NOTE: this module should be removed in the next Solana upgrade.

use {
    base64::{prelude::BASE64_STANDARD, Engine},
    clap::ArgMatches,
    spl_token_2022::solana_zk_sdk::encryption::{
        elgamal::{ElGamalKeypair, ElGamalPubkey},
        pod::elgamal::PodElGamalPubkey,
    },
};

const ELGAMAL_PUBKEY_MAX_BASE64_LEN: usize = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElGamalPubkeyOrNone {
    ElGamalPubkey(PodElGamalPubkey),
    None,
}

impl From<ElGamalPubkeyOrNone> for Option<PodElGamalPubkey> {
    fn from(val: ElGamalPubkeyOrNone) -> Self {
        match val {
            ElGamalPubkeyOrNone::ElGamalPubkey(pubkey) => Some(pubkey),
            ElGamalPubkeyOrNone::None => None,
        }
    }
}

pub(crate) fn elgamal_pubkey_or_none(
    matches: &ArgMatches,
    name: &str,
) -> Result<ElGamalPubkeyOrNone, String> {
    let arg_str = matches.value_of(name).unwrap();
    if arg_str == "none" {
        return Ok(ElGamalPubkeyOrNone::None);
    }
    let elgamal_pubkey = elgamal_pubkey_of(matches, name)?;
    Ok(ElGamalPubkeyOrNone::ElGamalPubkey(elgamal_pubkey))
}

pub(crate) fn elgamal_pubkey_of(
    matches: &ArgMatches,
    name: &str,
) -> Result<PodElGamalPubkey, String> {
    if let Ok(keypair) = elgamal_keypair_of(matches, name) {
        let elgamal_pubkey = (*keypair.pubkey()).into();
        Ok(elgamal_pubkey)
    } else {
        let arg_str = matches.value_of(name).unwrap();
        if let Some(pubkey) = elgamal_pubkey_from_str(arg_str) {
            Ok(pubkey)
        } else {
            Err("failed to read ElGamal pubkey".to_string())
        }
    }
}

pub(crate) fn elgamal_keypair_of(
    matches: &ArgMatches,
    name: &str,
) -> Result<ElGamalKeypair, String> {
    let path = matches.value_of(name).unwrap();
    ElGamalKeypair::read_json_file(path).map_err(|e| e.to_string())
}

fn elgamal_pubkey_from_str(s: &str) -> Option<PodElGamalPubkey> {
    if s.len() > ELGAMAL_PUBKEY_MAX_BASE64_LEN {
        return None;
    }
    let pubkey_vec = BASE64_STANDARD.decode(s).ok()?;
    let elgamal_pubkey = ElGamalPubkey::try_from(pubkey_vec.as_ref()).ok()?;
    Some(elgamal_pubkey.into())
}
