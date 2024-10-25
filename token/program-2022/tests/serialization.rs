#![cfg(feature = "serde-traits")]

use {
    base64::{engine::general_purpose::STANDARD, Engine},
    solana_program::program_option::COption,
    solana_sdk::pubkey::Pubkey,
    spl_pod::optional_keys::{OptionalNonZeroElGamalPubkey, OptionalNonZeroPubkey},
    spl_token_2022::{extension::confidential_transfer, instruction},
    std::str::FromStr,
};

#[test]
fn serde_instruction_coption_pubkey() {
    let inst = instruction::TokenInstruction::InitializeMint2 {
        decimals: 0,
        mint_authority: Pubkey::from_str("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM").unwrap(),
        freeze_authority: COption::Some(
            Pubkey::from_str("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap(),
        ),
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    assert_eq!(&serialized, "{\"initializeMint2\":{\"decimals\":0,\"mintAuthority\":\"4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM\",\"freezeAuthority\":\"8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh\"}}");

    serde_json::from_str::<instruction::TokenInstruction>(&serialized).unwrap();
}

#[test]
fn serde_instruction_coption_pubkey_with_none() {
    let inst = instruction::TokenInstruction::InitializeMintCloseAuthority {
        close_authority: COption::None,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    assert_eq!(
        &serialized,
        "{\"initializeMintCloseAuthority\":{\"closeAuthority\":null}}"
    );

    serde_json::from_str::<instruction::TokenInstruction>(&serialized).unwrap();
}

#[test]
fn serde_instruction_optional_nonzero_pubkeys_podbool() {
    // tests serde of ix containing OptionalNonZeroPubkey, PodBool and
    // OptionalNonZeroElGamalPubkey
    let authority_option: Option<Pubkey> =
        Some(Pubkey::from_str("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM").unwrap());
    let authority: OptionalNonZeroPubkey = authority_option.try_into().unwrap();

    let pubkey_string = STANDARD.encode([
        162, 23, 108, 36, 130, 143, 18, 219, 196, 134, 242, 145, 179, 49, 229, 193, 74, 64, 3, 158,
        68, 235, 124, 88, 247, 144, 164, 254, 228, 12, 173, 85,
    ]);
    let elgamal_pubkey_pod_option = Some(FromStr::from_str(&pubkey_string).unwrap());

    let auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey =
        elgamal_pubkey_pod_option.try_into().unwrap();

    let inst = confidential_transfer::instruction::InitializeMintData {
        authority,
        auto_approve_new_accounts: false.into(),
        auditor_elgamal_pubkey,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected = &"{\"authority\":\"4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM\",\"autoApproveNewAccounts\":false,\"auditorElgamalPubkey\":\"ohdsJIKPEtvEhvKRszHlwUpAA55E63xY95Ck/uQMrVU=\"}";
    assert_eq!(&serialized, serialized_expected);

    let deserialized =
        serde_json::from_str::<confidential_transfer::instruction::InitializeMintData>(&serialized)
            .unwrap();
    assert_eq!(inst, deserialized);
}

#[test]
fn serde_instruction_optional_nonzero_pubkeys_podbool_with_none() {
    // tests serde of ix containing OptionalNonZeroPubkey, PodBool and
    // OptionalNonZeroElGamalPubkey with null values
    let authority: OptionalNonZeroPubkey = None.try_into().unwrap();

    let auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey =
        OptionalNonZeroElGamalPubkey::default();

    let inst = confidential_transfer::instruction::InitializeMintData {
        authority,
        auto_approve_new_accounts: false.into(),
        auditor_elgamal_pubkey,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected =
        &"{\"authority\":null,\"autoApproveNewAccounts\":false,\"auditorElgamalPubkey\":null}";
    assert_eq!(&serialized, serialized_expected);

    let deserialized =
        serde_json::from_str::<confidential_transfer::instruction::InitializeMintData>(
            serialized_expected,
        )
        .unwrap();
    assert_eq!(inst, deserialized);
}

#[test]
fn serde_instruction_decryptable_balance_podu64() {
    let ciphertext_string = STANDARD.encode([
        56, 22, 102, 48, 112, 106, 58, 25, 25, 244, 194, 217, 73, 137, 73, 38, 24, 26, 36, 25, 235,
        234, 68, 181, 11, 82, 170, 163, 89, 205, 113, 160, 55, 16, 35, 151,
    ]);
    let decryptable_zero_balance = FromStr::from_str(&ciphertext_string).unwrap();

    let inst = confidential_transfer::instruction::ConfigureAccountInstructionData {
        decryptable_zero_balance,
        maximum_pending_balance_credit_counter: 1099.into(),
        proof_instruction_offset: 100,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected = &"{\"decryptableZeroBalance\":\"OBZmMHBqOhkZ9MLZSYlJJhgaJBnr6kS1C1Kqo1nNcaA3ECOX\",\"maximumPendingBalanceCreditCounter\":1099,\"proofInstructionOffset\":100}";
    assert_eq!(&serialized, serialized_expected);

    let deserialized = serde_json::from_str::<
        confidential_transfer::instruction::ConfigureAccountInstructionData,
    >(serialized_expected)
    .unwrap();
    assert_eq!(inst, deserialized);
}

#[test]
fn serde_instruction_elgamal_pubkey() {
    use spl_token_2022::extension::confidential_transfer_fee::instruction::InitializeConfidentialTransferFeeConfigData;

    let pubkey_string = STANDARD.encode([
        162, 23, 108, 36, 130, 143, 18, 219, 196, 134, 242, 145, 179, 49, 229, 193, 74, 64, 3, 158,
        68, 235, 124, 88, 247, 144, 164, 254, 228, 12, 173, 85,
    ]);
    let withdraw_withheld_authority_elgamal_pubkey = FromStr::from_str(&pubkey_string).unwrap();

    let inst = InitializeConfidentialTransferFeeConfigData {
        authority: OptionalNonZeroPubkey::default(),
        withdraw_withheld_authority_elgamal_pubkey,
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected = "{\"authority\":null,\"withdrawWithheldAuthorityElgamalPubkey\":\"ohdsJIKPEtvEhvKRszHlwUpAA55E63xY95Ck/uQMrVU=\"}";
    assert_eq!(&serialized, serialized_expected);

    let deserialized =
        serde_json::from_str::<InitializeConfidentialTransferFeeConfigData>(serialized_expected)
            .unwrap();
    assert_eq!(inst, deserialized);
}

#[test]
fn serde_instruction_basis_points() {
    use spl_token_2022::extension::interest_bearing_mint::instruction::InitializeInstructionData;

    let inst = InitializeInstructionData {
        rate_authority: OptionalNonZeroPubkey::default(),
        rate: 127.into(),
    };

    let serialized = serde_json::to_string(&inst).unwrap();
    let serialized_expected = "{\"rateAuthority\":null,\"rate\":127}";
    assert_eq!(&serialized, serialized_expected);

    serde_json::from_str::<InitializeInstructionData>(serialized_expected).unwrap();
}
