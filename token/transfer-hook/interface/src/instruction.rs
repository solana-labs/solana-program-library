//! Instruction types

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_pod::{bytemuck::pod_slice_to_bytes, slice::PodSlice},
    spl_tlv_account_resolution::account::ExtraAccountMeta,
    std::convert::TryInto,
};

/// Instructions supported by the transfer hook interface.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum TransferHookInstruction {
    /// Runs additional transfer logic.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` Source account
    ///   1. `[]` Token mint
    ///   2. `[]` Destination account
    ///   3. `[]` Source account's owner/delegate
    ///   4. `[]` (Optional) Validation account
    ///   5..5+M `[]` `M` optional additional accounts, written in validation
    /// account     data
    Execute {
        /// Amount of tokens to transfer
        amount: u64,
    },

    /// Initializes the extra account metas on an account, writing into the
    /// first open TLV space.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Account with extra account metas
    ///   1. `[]` Mint
    ///   2. `[s]` Mint authority
    ///   3. `[]` System program
    InitializeExtraAccountMetaList {
        /// List of `ExtraAccountMeta`s to write into the account
        extra_account_metas: Vec<ExtraAccountMeta>,
    },
    /// Updates the extra account metas on an account by overwriting the
    /// existing list.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Account with extra account metas
    ///   1. `[]` Mint
    ///   2. `[s]` Mint authority
    UpdateExtraAccountMetaList {
        /// The new list of `ExtraAccountMetas` to overwrite the existing entry
        /// in the account.
        extra_account_metas: Vec<ExtraAccountMeta>,
    },
}
/// TLV instruction type only used to define the discriminator. The actual data
/// is entirely managed by `ExtraAccountMetaList`, and it is the only data
/// contained by this type.
#[derive(SplDiscriminate)]
#[discriminator_hash_input("spl-transfer-hook-interface:execute")]
pub struct ExecuteInstruction;

/// TLV instruction type used to initialize extra account metas
/// for the transfer hook
#[derive(SplDiscriminate)]
#[discriminator_hash_input("spl-transfer-hook-interface:initialize-extra-account-metas")]
pub struct InitializeExtraAccountMetaListInstruction;

/// TLV instruction type used to update extra account metas
/// for the transfer hook
#[derive(SplDiscriminate)]
#[discriminator_hash_input("spl-transfer-hook-interface:update-extra-account-metas")]
pub struct UpdateExtraAccountMetaListInstruction;

impl TransferHookInstruction {
    /// Unpacks a byte buffer into a
    /// [TransferHookInstruction](enum.TransferHookInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(ArrayDiscriminator::LENGTH);
        Ok(match discriminator {
            ExecuteInstruction::SPL_DISCRIMINATOR_SLICE => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::Execute { amount }
            }
            InitializeExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE => {
                let pod_slice = PodSlice::<ExtraAccountMeta>::unpack(rest)?;
                let extra_account_metas = pod_slice.data().to_vec();
                Self::InitializeExtraAccountMetaList {
                    extra_account_metas,
                }
            }
            UpdateExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE => {
                let pod_slice = PodSlice::<ExtraAccountMeta>::unpack(rest)?;
                let extra_account_metas = pod_slice.data().to_vec();
                Self::UpdateExtraAccountMetaList {
                    extra_account_metas,
                }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [TokenInstruction](enum.TokenInstruction.html) into a byte
    /// buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::Execute { amount } => {
                buf.extend_from_slice(ExecuteInstruction::SPL_DISCRIMINATOR_SLICE);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::InitializeExtraAccountMetaList {
                extra_account_metas,
            } => {
                buf.extend_from_slice(
                    InitializeExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE,
                );
                buf.extend_from_slice(&(extra_account_metas.len() as u32).to_le_bytes());
                buf.extend_from_slice(pod_slice_to_bytes(extra_account_metas));
            }
            Self::UpdateExtraAccountMetaList {
                extra_account_metas,
            } => {
                buf.extend_from_slice(
                    UpdateExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE,
                );
                buf.extend_from_slice(&(extra_account_metas.len() as u32).to_le_bytes());
                buf.extend_from_slice(pod_slice_to_bytes(extra_account_metas));
            }
        };
        buf
    }
}

/// Creates an `Execute` instruction, provided all of the additional required
/// account metas
#[allow(clippy::too_many_arguments)]
pub fn execute_with_extra_account_metas(
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    validate_state_pubkey: &Pubkey,
    additional_accounts: &[AccountMeta],
    amount: u64,
) -> Instruction {
    let mut instruction = execute(
        program_id,
        source_pubkey,
        mint_pubkey,
        destination_pubkey,
        authority_pubkey,
        amount,
    );
    instruction
        .accounts
        .push(AccountMeta::new_readonly(*validate_state_pubkey, false));
    instruction.accounts.extend_from_slice(additional_accounts);
    instruction
}

/// Creates an `Execute` instruction, without the additional accounts
#[allow(clippy::too_many_arguments)]
pub fn execute(
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    amount: u64,
) -> Instruction {
    let data = TransferHookInstruction::Execute { amount }.pack();
    let accounts = vec![
        AccountMeta::new_readonly(*source_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*destination_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `InitializeExtraAccountMetaList` instruction.
pub fn initialize_extra_account_meta_list(
    program_id: &Pubkey,
    extra_account_metas_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    extra_account_metas: &[ExtraAccountMeta],
) -> Instruction {
    let data = TransferHookInstruction::InitializeExtraAccountMetaList {
        extra_account_metas: extra_account_metas.to_vec(),
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*extra_account_metas_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `UpdateExtraAccountMetaList` instruction.
pub fn update_extra_account_meta_list(
    program_id: &Pubkey,
    extra_account_metas_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    extra_account_metas: &[ExtraAccountMeta],
) -> Instruction {
    let data = TransferHookInstruction::UpdateExtraAccountMetaList {
        extra_account_metas: extra_account_metas.to_vec(),
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*extra_account_metas_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, true),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::NAMESPACE, solana_program::hash, spl_pod::bytemuck::pod_from_bytes};

    #[test]
    fn validate_packing() {
        let amount = 111_111_111;
        let check = TransferHookInstruction::Execute { amount };
        let packed = check.pack();
        // Please use ExecuteInstruction::SPL_DISCRIMINATOR in your program, the
        // following is just for test purposes
        let preimage = hash::hashv(&[format!("{NAMESPACE}:execute").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.extend_from_slice(&amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = TransferHookInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn initialize_validation_pubkeys_packing() {
        let extra_meta_len_bytes = &[
            1, 0, 0, 0, // `1u32`
        ];
        let extra_meta_bytes = &[
            0, // `AccountMeta`
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, // pubkey
            0, // is_signer
            0, // is_writable
        ];
        let extra_account_metas =
            vec![*pod_from_bytes::<ExtraAccountMeta>(extra_meta_bytes).unwrap()];
        let check = TransferHookInstruction::InitializeExtraAccountMetaList {
            extra_account_metas,
        };
        let packed = check.pack();
        // Please use INITIALIZE_EXTRA_ACCOUNT_METAS_DISCRIMINATOR in your program,
        // the following is just for test purposes
        let preimage =
            hash::hashv(&[format!("{NAMESPACE}:initialize-extra-account-metas").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.extend_from_slice(extra_meta_len_bytes);
        expect.extend_from_slice(extra_meta_bytes);
        assert_eq!(packed, expect);
        let unpacked = TransferHookInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
