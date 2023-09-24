use {
    base64::{prelude::BASE64_STANDARD, Engine},
    clap::ArgMatches,
    spl_token_2022::solana_zk_token_sdk::{
        encryption::elgamal::{ElGamalKeypair, ElGamalPubkey},
        zk_token_elgamal::pod::ElGamalPubkey as PodElGamalPubkey,
    },
};

const ELGAMAL_PUBKEY_MAX_BASE64_LEN: usize = 44;

pub(crate) fn elgamal_pubkey_or_none(
    matches: &ArgMatches,
    name: &str,
) -> Result<Option<PodElGamalPubkey>, String> {
    let arg_str = matches.value_of(name).unwrap();
    if arg_str == "none" {
        return Ok(None);
    }
    elgamal_pubkey_of(matches, name).map(Some)
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
    let elgamal_pubkey = ElGamalPubkey::from_bytes(&pubkey_vec)?;
    Some(elgamal_pubkey.into())
}
