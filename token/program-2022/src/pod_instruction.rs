//! Rewrites of the instruction data types represented as Pods

use {
    crate::pod::PodCOption,
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        program_error::ProgramError,
        pubkey::{Pubkey, PUBKEY_BYTES},
    },
    spl_pod::{
        bytemuck::{pod_from_bytes, pod_get_packed_len},
        primitives::PodU64,
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct InitializeMintData {
    /// Number of base 10 digits to the right of the decimal place.
    pub(crate) decimals: u8,
    /// The authority/multisignature to mint tokens.
    pub(crate) mint_authority: Pubkey,
    // The freeze authority option comes later, but cannot be included as
    // plain old data in this struct
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct InitializeMultisigData {
    /// The number of signers (M) required to validate this multisignature
    /// account.
    pub(crate) m: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct AmountData {
    /// The amount of tokens to transfer.
    pub(crate) amount: PodU64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct AmountCheckedData {
    /// The amount of tokens to transfer.
    pub(crate) amount: PodU64,
    /// Decimals of the mint
    pub(crate) decimals: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct SetAuthorityData {
    /// The type of authority to update.
    pub(crate) authority_type: u8,
    // The new authority option comes later, but cannot be included as
    // plain old data in this struct
}

/// All of the base instructions in Token-2022, reduced down to their one-byte
/// discriminant.
///
/// All instructions that expect data afterwards include a comment with the data
/// type expected. For example, `PodTokenInstruction::InitializeMint` expects
/// `InitializeMintData`.
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub(crate) enum PodTokenInstruction {
    // 0
    InitializeMint, // InitializeMintData
    InitializeAccount,
    InitializeMultisig, // InitializeMultisigData
    Transfer,           // AmountData
    Approve,            // AmountData
    // 5
    Revoke,
    SetAuthority, // SetAuthorityData
    MintTo,       // AmountData
    Burn,         // AmountData
    CloseAccount,
    // 10
    FreezeAccount,
    ThawAccount,
    TransferChecked, // AmountCheckedData
    ApproveChecked,  // AmountCheckedData
    MintToChecked,   // AmountCheckedData
    // 15
    BurnChecked,        // AmountCheckedData
    InitializeAccount2, // Pubkey
    SyncNative,
    InitializeAccount3,  // Pubkey
    InitializeMultisig2, // InitializeMultisigData
    // 20
    InitializeMint2,    // InitializeMintData
    GetAccountDataSize, // &[ExtensionType]
    InitializeImmutableOwner,
    AmountToUiAmount, // AmountData
    UiAmountToAmount, // &str
    // 25
    InitializeMintCloseAuthority, // COption<Pubkey>
    TransferFeeExtension,
    ConfidentialTransferExtension,
    DefaultAccountStateExtension,
    Reallocate, // &[ExtensionType]
    // 30
    MemoTransferExtension,
    CreateNativeMint,
    InitializeNonTransferableMint,
    InterestBearingMintExtension,
    CpiGuardExtension,
    // 35
    InitializePermanentDelegate, // Pubkey
    TransferHookExtension,
    ConfidentialTransferFeeExtension,
    WithdrawExcessLamports,
    MetadataPointerExtension,
    // 40
    GroupPointerExtension,
    GroupMemberPointerExtension,
}

fn unpack_pubkey_option(input: &[u8]) -> Result<PodCOption<Pubkey>, ProgramError> {
    match input.split_first() {
        Option::Some((&0, _)) => Ok(PodCOption::none()),
        Option::Some((&1, rest)) => {
            let pk = rest
                .get(..PUBKEY_BYTES)
                .and_then(|x| Pubkey::try_from(x).ok())
                .ok_or(ProgramError::InvalidInstructionData)?;
            Ok(PodCOption::some(pk))
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Specialty function for deserializing `Pod` data and a `COption<Pubkey>`
///
/// `COption<T>` is not `Pod` compatible when serialized in an instruction, but
/// since it is always at the end of an instruction, so we can do this safely
pub(crate) fn decode_instruction_data_with_coption_pubkey<T: Pod>(
    input_with_type: &[u8],
) -> Result<(&T, PodCOption<Pubkey>), ProgramError> {
    let end_of_t = pod_get_packed_len::<T>().saturating_add(1);
    let value = input_with_type
        .get(1..end_of_t)
        .ok_or(ProgramError::InvalidInstructionData)
        .and_then(pod_from_bytes)?;
    let pubkey = unpack_pubkey_option(&input_with_type[end_of_t..])?;
    Ok((value, pubkey))
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            extension::ExtensionType,
            instruction::{decode_instruction_data, decode_instruction_type},
        },
        proptest::prelude::*,
    };

    // Test function that mimics the "unpacking" in `Processor::process` by
    // trying to deserialize the relevant type data after the instruction type
    fn check_pod_instruction(input: &[u8]) -> Result<(), ProgramError> {
        if let Ok(instruction_type) = decode_instruction_type(input) {
            match instruction_type {
                PodTokenInstruction::InitializeMint | PodTokenInstruction::InitializeMint2 => {
                    let _ =
                        decode_instruction_data_with_coption_pubkey::<InitializeMintData>(input)?;
                }
                PodTokenInstruction::InitializeAccount2
                | PodTokenInstruction::InitializeAccount3
                | PodTokenInstruction::InitializePermanentDelegate => {
                    let _ = decode_instruction_data::<Pubkey>(input)?;
                }
                PodTokenInstruction::InitializeMultisig
                | PodTokenInstruction::InitializeMultisig2 => {
                    let _ = decode_instruction_data::<InitializeMultisigData>(input)?;
                }
                PodTokenInstruction::SetAuthority => {
                    let _ = decode_instruction_data_with_coption_pubkey::<SetAuthorityData>(input)?;
                }
                PodTokenInstruction::Transfer
                | PodTokenInstruction::Approve
                | PodTokenInstruction::MintTo
                | PodTokenInstruction::Burn
                | PodTokenInstruction::AmountToUiAmount => {
                    let _ = decode_instruction_data::<AmountData>(input)?;
                }
                PodTokenInstruction::TransferChecked
                | PodTokenInstruction::ApproveChecked
                | PodTokenInstruction::MintToChecked
                | PodTokenInstruction::BurnChecked => {
                    let _ = decode_instruction_data::<AmountCheckedData>(input)?;
                }
                PodTokenInstruction::InitializeMintCloseAuthority => {
                    let _ = decode_instruction_data_with_coption_pubkey::<()>(input)?;
                }
                PodTokenInstruction::UiAmountToAmount => {
                    let _ = std::str::from_utf8(&input[1..])
                        .map_err(|_| ProgramError::InvalidInstructionData)?;
                }
                PodTokenInstruction::GetAccountDataSize | PodTokenInstruction::Reallocate => {
                    let _ = input[1..]
                        .chunks(std::mem::size_of::<ExtensionType>())
                        .map(ExtensionType::try_from)
                        .collect::<Result<Vec<_>, _>>()?;
                }
                _ => {
                    // no extra data to deserialize
                }
            }
        }
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]
        #[test]
        fn test_instruction_unpack_proptest(
            data in prop::collection::vec(any::<u8>(), 0..255)
        ) {
            let _no_panic = check_pod_instruction(&data);
        }
    }
}
