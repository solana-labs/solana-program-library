#[cfg(not(target_os = "solana"))]
use crate::{
    error::TokenError,
    extension::confidential_transfer::processor::verify_and_split_deposit_amount,
    proof::ProofLocation,
};
#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::{
    encryption::pedersen::Pedersen,
    instruction::{transfer::TransferAmountCiphertext, BatchedRangeProofU64Data},
    zk_token_proof_instruction::verify_batched_verify_range_proof_u64,
};

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::encryption::{elgamal::ElGamalPubkey, pedersen::PedersenOpening};
use {
    crate::extension::confidential_transfer::{
        ciphertext_extraction::SourceDecryptHandles, DecryptableBalance,
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::pubkey::Pubkey,
    solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalCiphertext,
};
#[cfg(not(target_os = "solana"))]
use {
    crate::{
        check_program_account,
        extension::confidential_transfer::instruction::TransferSplitContextStateAccounts,
        instruction::{encode_instruction, TokenInstruction},
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        sysvar,
    },
};

/// Confidential Transfer extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialMintBurnInstruction {
    /// Initializes confidential mints and burns for a mint.
    ///
    /// The `ConfidentialMintBurnInstruction::InitializeMint` instruction
    /// requires no signers and MUST be included within the same Transaction
    /// as `TokenInstruction::InitializeMint`. Otherwise another party can
    /// initialize the configuration.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint`
    /// instruction has already executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///
    /// Data expected by this instruction:
    ///   `InitializeMintData`
    InitializeMint,
    /// Updates mint-authority for confidential-mint-burn mint.
    UpdateMint,
    /// Mints confidential tokens to
    ConfidentialMint,
    /// Removes whitelist designation for specific address
    ConfidentialBurn,
}

/// Data expected by `WhitelistedTransferInstruction::InitializeMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Authority to modify the `WhitelistTransferMint` configuration and to
    /// approve new accounts.
    pub authority: Pubkey,
}

/// Data expected by `ConfidentialTransferInstruction::UpdateMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateMintData {
    /// Determines if newly configured accounts must be approved by the
    /// `authority` before they may be used by the user.
    pub new_authority: Pubkey,
}

/// Data expected by `ConfidentialMintBurnInstruction::ConfidentialMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct MintInstructionData {
    /// low 16 bits of encrypted amount to be minted
    pub mint_lo: ElGamalCiphertext,
    /// high 48 bits of encrypted amount to be minted
    pub mint_hi: ElGamalCiphertext,
    /// low 16 bits of encrypted amount to be minted
    pub audit_amount_lo: ElGamalCiphertext,
    /// high 48 bits of encrypted amount to be minted
    pub audit_amount_hi: ElGamalCiphertext,
    /// Relative location of the `ProofInstruction::VerifyBatchedRangeProofU64`
    /// instruction to the `ConfidentialMint` instruction in the
    /// transaction.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialMintBurnInstruction::ConfidentialBurn`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct BurnInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_available_balance: DecryptableBalance,
    /// The ElGamal decryption handle pertaining to the low and high bits of the
    /// transfer amount. This field is used when the transfer proofs are
    /// split and verified as smaller components.
    ///
    /// NOTE: This field is to be removed in the next Solana upgrade.
    pub source_decrypt_handles: SourceDecryptHandles,
    /// low 16 bits of encrypted amount to be minted
    pub burn_lo: ElGamalCiphertext,
    /// high 48 bits of encrypted amount to be minted
    pub burn_hi: ElGamalCiphertext,
}

/// Create a `InitializeMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::InitializeMint,
        &InitializeMintData { authority },
    ))
}

/// Create a `UpdateMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn update_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    new_authority: Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::UpdateMint,
        &UpdateMintData { new_authority },
    ))
}

/// Create a `ConfidentialMint` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn confidential_mint(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: Option<ElGamalPubkey>,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<BatchedRangeProofU64Data>,
) -> Result<Vec<Instruction>, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let (amount_lo, amount_hi) = verify_and_split_deposit_amount(amount)?;
    let opening = PedersenOpening::new_rand();
    let mint_lo = destination_elgamal_pubkey.encrypt_with(amount_lo, &opening);
    let mint_hi = destination_elgamal_pubkey.encrypt_with(amount_hi, &opening);

    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();
    let opening = PedersenOpening::new_rand();
    let audit_amount_hi = auditor_elgamal_pubkey.encrypt_with(amount_hi, &opening);
    let audit_amount_lo = auditor_elgamal_pubkey.encrypt_with(amount_lo, &opening);

    let proof_instruction_offset = match proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, _) => {
            accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
            proof_instruction_offset.into()
        }
        ProofLocation::ContextStateAccount(context_state_account) => {
            accounts.push(AccountMeta::new_readonly(*context_state_account, false));
            0
        }
    };

    let mut instrs = vec![encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::ConfidentialMint,
        &MintInstructionData {
            mint_lo: mint_lo.into(),
            mint_hi: mint_hi.into(),
            audit_amount_lo: audit_amount_lo.into(),
            audit_amount_hi: audit_amount_hi.into(),
            proof_instruction_offset,
        },
    )];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the
        // `ConfidentialMint` instruction. This means that the proof instruction
        // offset must be always be 1.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instrs.push(verify_batched_verify_range_proof_u64(
            None,
            proof_data,
        ))
    };

    Ok(instrs)
}

/// Generates range proof for mint instruction
#[cfg(not(target_os = "solana"))]
pub fn mint_range_proof(
    amount: u64,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: &Option<ElGamalPubkey>,
) -> Result<BatchedRangeProofU64Data, ProgramError> {
    let (amount_lo, amount_hi) = verify_and_split_deposit_amount(amount)?;
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();

    const MINT_AMOUNT_LO_BIT_LENGTH: usize = 16;
    const MINT_AMOUNT_HI_BIT_LENGTH: usize = 32;
    const PADDING_BIT_LENGTH: usize = 16;

    // Encrypt the `lo` and `hi` transfer amounts.
    let (mint_amount_grouped_ciphertext_lo, transfer_amount_opening_lo) =
        TransferAmountCiphertext::new(
            amount_lo,
            destination_elgamal_pubkey,
            &ElGamalPubkey::default(),
            &auditor_elgamal_pubkey,
        );

    let (transfer_amount_grouped_ciphertext_hi, transfer_amount_opening_hi) =
        TransferAmountCiphertext::new(
            amount_hi,
            destination_elgamal_pubkey,
            &ElGamalPubkey::default(),
            &auditor_elgamal_pubkey,
        );

    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);

    Ok(BatchedRangeProofU64Data::new(
        vec![
            mint_amount_grouped_ciphertext_lo.get_commitment(),
            transfer_amount_grouped_ciphertext_hi.get_commitment(),
            &padding_commitment,
        ],
        vec![amount_lo, amount_hi, 0],
        vec![
            MINT_AMOUNT_LO_BIT_LENGTH,
            MINT_AMOUNT_HI_BIT_LENGTH,
            PADDING_BIT_LENGTH,
        ],
        vec![
            &transfer_amount_opening_lo,
            &transfer_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(|_| TokenError::ProofGeneration)?)
}

/// Create a `ConfidentialBurn` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn confidential_burn_with_split_proofs(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    auditor_pubkey: Option<ElGamalPubkey>,
    burn_amount: u64,
    new_decryptable_available_balance: DecryptableBalance,
    context_accounts: TransferSplitContextStateAccounts,
    source_decrypt_handles: &SourceDecryptHandles,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    pedersen_openings: &(PedersenOpening, PedersenOpening),
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![inner_confidential_burn_with_split_proofs(
        token_program_id,
        token_account,
        mint,
        auditor_pubkey,
        burn_amount,
        new_decryptable_available_balance,
        context_accounts,
        source_decrypt_handles,
        authority,
        multisig_signers,
        pedersen_openings,
    )?])
}

/// Create a inner `ConfidentialBurn` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn inner_confidential_burn_with_split_proofs(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    auditor_pubkey: Option<ElGamalPubkey>,
    burn_amount: u64,
    new_decryptable_available_balance: DecryptableBalance,
    context_accounts: TransferSplitContextStateAccounts,
    source_decrypt_handles: &SourceDecryptHandles,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    pedersen_openings: &(PedersenOpening, PedersenOpening),
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
    ];

    if context_accounts
        .close_split_context_state_accounts
        .is_some()
    {
        println!("close split context accounts on execution not implemented for confidential burn");
        return Err(ProgramError::InvalidInstructionData);
    }

    accounts.push(AccountMeta::new_readonly(
        *context_accounts.equality_proof,
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        *context_accounts.ciphertext_validity_proof,
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        *context_accounts.range_proof,
        false,
    ));

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let (burn_hi, burn_lo) = if let Some(apk) = auditor_pubkey {
        let (opening_hi, opening_lo) = pedersen_openings;
        let (amount_lo, amount_hi) = verify_and_split_deposit_amount(burn_amount)?;
        let burn_hi = apk.encrypt_with(amount_hi, opening_hi);
        let burn_lo = apk.encrypt_with(amount_lo, opening_lo);
        (burn_hi.into(), burn_lo.into())
    } else {
        (ElGamalCiphertext::zeroed(), ElGamalCiphertext::zeroed())
    };

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::ConfidentialBurn,
        &BurnInstructionData {
            new_decryptable_available_balance,
            source_decrypt_handles: *source_decrypt_handles,
            burn_hi,
            burn_lo,
        },
    ))
}
