#![cfg(feature = "serde-traits")]

use {
    solana_program::program_option::COption, solana_sdk::pubkey::Pubkey,
    spl_token_2022::instruction, std::str::FromStr,
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
