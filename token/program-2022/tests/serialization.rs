#![cfg(feature = "serde-traits")]

use {
    solana_program::program_option::COption,
    solana_sdk::{
        pubkey::Pubkey,
    },
    spl_token_2022::{
        extension::{confidential_transfer},
        instruction,
        solana_zk_token_sdk::{
            encryption::elgamal::{
                ElGamalPubkey as ElGamalPubkeyDecoded,
                ElGamalSecretKey as ElGamalSecretKeyDecoded,
            },
            zk_token_elgamal::pod::ElGamalPubkey as ElGamalPubkeyPod,
        },
        pod::{OptionalNonZeroPubkey, OptionalNonZeroElGamalPubkey, PodBool},
    },
    std::str::FromStr,
};

#[test]
fn token_program_serde() {
    let inst = instruction::TokenInstruction::InitializeMint2 {
        decimals: 0,
        mint_authority: Pubkey::from_str("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM").unwrap(),
        freeze_authority: COption::Some(
            Pubkey::from_str("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap(),
        ),
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    assert_eq!(&serialized, "{\"InitializeMint2\":{\"decimals\":0,\"mint_authority\":\"4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM\",\"freeze_authority\":\"8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh\"}}");

    serde_json::from_str::<instruction::TokenInstruction>(&serialized).unwrap();
}

#[test]
fn token_program_serde_with_none() {
    let inst = instruction::TokenInstruction::InitializeMintCloseAuthority {
        close_authority: COption::None,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    assert_eq!(
        &serialized,
        "{\"InitializeMintCloseAuthority\":{\"close_authority\":null}}"
    );

    serde_json::from_str::<instruction::TokenInstruction>(&serialized).unwrap();
}

#[test]
fn token_program_extension_serde() {
    let authority_option: Option<Pubkey> = Some(Pubkey::from_str("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM").unwrap());
    let authority: OptionalNonZeroPubkey = authority_option.try_into().unwrap();

    let elgamal_secretkey: ElGamalSecretKeyDecoded = ElGamalSecretKeyDecoded::new_rand();
    let elgamal_pubkey: ElGamalPubkeyDecoded = ElGamalPubkeyDecoded::new(&elgamal_secretkey);
    let elgamal_pubkey_pod: ElGamalPubkeyPod = ElGamalPubkeyPod::from(elgamal_pubkey);
    let elgamal_pubkey_pod_option: Option<ElGamalPubkeyPod> = Some(elgamal_pubkey_pod);
    let auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey = elgamal_pubkey_pod_option.try_into().unwrap();

    let inst = confidential_transfer::instruction::InitializeMintData {
        authority,
        auto_approve_new_accounts: false.into(),
        auditor_elgamal_pubkey,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected = &format!("{{\"authority\":\"4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM\",\"auto_approve_new_accounts\":false,\"auditor_elgamal_pubkey\":\"{}\"}}", elgamal_pubkey.to_string());
    assert_eq!(&serialized, serialized_expected);

    let deserialized = serde_json::from_str::<confidential_transfer::instruction::InitializeMintData>(&serialized).unwrap();
    assert_eq!(inst, deserialized);
}

#[test]
fn token_program_extension_serde_with_none() {
    let authority: OptionalNonZeroPubkey = None.try_into().unwrap();

    let auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey = OptionalNonZeroElGamalPubkey::default();

    let inst = confidential_transfer::instruction::InitializeMintData {
        authority,
        auto_approve_new_accounts: false.into(),
        auditor_elgamal_pubkey,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected = &format!("{{\"authority\":null,\"auto_approve_new_accounts\":false,\"auditor_elgamal_pubkey\":null}}");
    assert_eq!(&serialized, serialized_expected);

    let deserialized = serde_json::from_str::<confidential_transfer::instruction::InitializeMintData>(&serialized_expected).unwrap();
    assert_eq!(inst, deserialized);
}
