use {
    crate::{error::TokenError, id, instruction::TokenInstruction},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    std::convert::TryInto,
};

/// Transfer Fee extension instructions
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum TransferFeeInstruction {
    /// Initialize the transfer fee on a new mint.
    ///
    /// Fails if the mint has already been initialized, so must be called before
    /// `InitializeMint`.
    ///
    /// The mint must have exactly enough space allocated for the base mint (82
    /// bytes), plus 83 bytes of padding, 1 byte reserved for the account type,
    /// then space required for this extension, plus any others.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to initialize.
    InitializeTransferFeeConfig {
        /// Pubkey that may update the fees
        transfer_fee_config_authority: COption<Pubkey>,
        /// Withdraw instructions must be signed by this key
        withdraw_withheld_authority: COption<Pubkey>,
        /// Amount of transfer collected as fees, expressed as basis points of the
        /// transfer amount
        transfer_fee_basis_points: u16,
        /// Maximum fee assessed on transfers
        maximum_fee: u64,
    },
    /// Transfer, providing expected mint information and fees
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The source account. Must include the `TransferFeeAmount` extension.
    ///   1. `[]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   2. `[writable]` The destination account. Must include the `TransferFeeAmount` extension.
    ///   3. `[signer]` The source account's owner/delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The source account.
    ///   1. `[]` The token mint.
    ///   2. `[writable]` The destination account.
    ///   3. `[]` The source account's multisignature owner/delegate.
    ///   4. ..4+M `[signer]` M signer accounts.
    TransferCheckedWithFee {
        /// The amount of tokens to transfer.
        amount: u64,
        /// Expected number of base 10 digits to the right of the decimal place.
        decimals: u8,
        /// Expected fee assessed on this transfer, calculated off-chain based on
        /// the transfer_fee_basis_points and maximum_fee of the mint.
        fee: u64,
    },
    /// Transfer all withheld tokens in the mint to an account. Signed by the mint's
    /// withdraw withheld tokens authority.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   1. `[writable]` The fee receiver account. Must include the `TransferFeeAmount` extension
    ///      associated with the provided mint.
    ///   2. `[signer]` The mint's `withdraw_withheld_authority`.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The token mint.
    ///   1. `[writable]` The destination account.
    ///   2. `[]` The mint's `withdraw_withheld_authority`'s multisignature owner/delegate.
    ///   3. ..3+M `[signer]` M signer accounts.
    WithdrawWithheldTokensFromMint,
    /// Transfer all withheld tokens to an account. Signed by the mint's
    /// withdraw withheld tokens authority.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   1. `[writable]` The fee receiver account. Must include the `TransferFeeAmount`
    ///      extension and be associated with the provided mint.
    ///   2. `[signer]` The mint's `withdraw_withheld_authority`.
    ///   3. ..3+N `[writable]` The source accounts to withdraw from.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[]` The token mint.
    ///   1. `[writable]` The destination account.
    ///   2. `[]` The mint's `withdraw_withheld_authority`'s multisignature owner/delegate.
    ///   3. ..3+M `[signer]` M signer accounts.
    ///   3+M+1. ..3+M+N `[writable]` The source accounts to withdraw from.
    WithdrawWithheldTokensFromAccounts,
    /// Permissionless instruction to transfer all withheld tokens to the mint.
    ///
    /// Succeeds for frozen accounts.
    ///
    /// Accounts provided should include the `TransferFeeAmount` extension. If not,
    /// the account is skipped.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint.
    ///   1. ..1+N `[writable]` The source accounts to harvest from.
    HarvestWithheldTokensToMint,
    /// Set transfer fee. Only supported for mints that include the `TransferFeeConfig` extension.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[signer]` The mint's fee account owner.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[]` The mint's multisignature fee account owner.
    ///   2. ..2+M `[signer]` M signer accounts.
    SetTransferFee {
        /// Amount of transfer collected as fees, expressed as basis points of the
        /// transfer amount
        transfer_fee_basis_points: u16,
        /// Maximum fee assessed on transfers
        maximum_fee: u64,
    },
}
impl TransferFeeInstruction {
    /// Unpacks a byte buffer into a TransferFeeInstruction
    pub fn unpack(input: &[u8]) -> Result<(Self, &[u8]), ProgramError> {
        use TokenError::InvalidInstruction;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let (transfer_fee_config_authority, rest) =
                    TokenInstruction::unpack_pubkey_option(rest)?;
                let (withdraw_withheld_authority, rest) =
                    TokenInstruction::unpack_pubkey_option(rest)?;
                let (transfer_fee_basis_points, rest) = rest.split_at(2);
                let transfer_fee_basis_points = transfer_fee_basis_points
                    .try_into()
                    .ok()
                    .map(u16::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (maximum_fee, rest) = rest.split_at(8);
                let maximum_fee = maximum_fee
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let instruction = Self::InitializeTransferFeeConfig {
                    transfer_fee_config_authority,
                    withdraw_withheld_authority,
                    transfer_fee_basis_points,
                    maximum_fee,
                };
                (instruction, rest)
            }
            1 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (&decimals, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let (fee, rest) = rest.split_at(8);
                let fee = fee
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let instruction = Self::TransferCheckedWithFee {
                    amount,
                    decimals,
                    fee,
                };
                (instruction, rest)
            }
            2 => (Self::WithdrawWithheldTokensFromMint, rest),
            3 => (Self::WithdrawWithheldTokensFromAccounts, rest),
            4 => (Self::HarvestWithheldTokensToMint, rest),
            5 => {
                let (transfer_fee_basis_points, rest) = rest.split_at(2);
                let transfer_fee_basis_points = transfer_fee_basis_points
                    .try_into()
                    .ok()
                    .map(u16::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (maximum_fee, rest) = rest.split_at(8);
                let maximum_fee = maximum_fee
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let instruction = Self::SetTransferFee {
                    transfer_fee_basis_points,
                    maximum_fee,
                };
                (instruction, rest)
            }
            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }

    /// Packs a TransferFeeInstruction into a byte buffer.
    pub fn pack(&self, buffer: &mut Vec<u8>) {
        match *self {
            Self::InitializeTransferFeeConfig {
                ref transfer_fee_config_authority,
                ref withdraw_withheld_authority,
                transfer_fee_basis_points,
                maximum_fee,
            } => {
                buffer.push(0);
                TokenInstruction::pack_pubkey_option(transfer_fee_config_authority, buffer);
                TokenInstruction::pack_pubkey_option(withdraw_withheld_authority, buffer);
                buffer.extend_from_slice(&transfer_fee_basis_points.to_le_bytes());
                buffer.extend_from_slice(&maximum_fee.to_le_bytes());
            }
            Self::TransferCheckedWithFee {
                amount,
                decimals,
                fee,
            } => {
                buffer.push(1);
                buffer.extend_from_slice(&amount.to_le_bytes());
                buffer.extend_from_slice(&decimals.to_le_bytes());
                buffer.extend_from_slice(&fee.to_le_bytes());
            }
            Self::WithdrawWithheldTokensFromMint => {
                buffer.push(2);
            }
            Self::WithdrawWithheldTokensFromAccounts => {
                buffer.push(3);
            }
            Self::HarvestWithheldTokensToMint => {
                buffer.push(4);
            }
            Self::SetTransferFee {
                transfer_fee_basis_points,
                maximum_fee,
            } => {
                buffer.push(5);
                buffer.extend_from_slice(&transfer_fee_basis_points.to_le_bytes());
                buffer.extend_from_slice(&maximum_fee.to_le_bytes());
            }
        }
    }
}

/// Create a `InitializeTransferFeeConfig` instruction
pub fn initialize_transfer_fee_config(
    mint: &Pubkey,
    transfer_fee_config_authority: Option<&Pubkey>,
    withdraw_withheld_authority: Option<&Pubkey>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Instruction {
    let transfer_fee_config_authority = transfer_fee_config_authority.cloned().into();
    let withdraw_withheld_authority = withdraw_withheld_authority.cloned().into();
    let data = TokenInstruction::TransferFeeExtension(
        TransferFeeInstruction::InitializeTransferFeeConfig {
            transfer_fee_config_authority,
            withdraw_withheld_authority,
            transfer_fee_basis_points,
            maximum_fee,
        },
    )
    .pack();

    Instruction {
        program_id: id(),
        accounts: vec![AccountMeta::new(*mint, false)],
        data,
    }
}

/// Create a `TransferCheckedWithFee` instruction
#[allow(clippy::too_many_arguments)]
pub fn transfer_checked_with_fee(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
    amount: u64,
    decimals: u8,
    fee: u64,
) -> Instruction {
    let data =
        TokenInstruction::TransferFeeExtension(TransferFeeInstruction::TransferCheckedWithFee {
            amount,
            decimals,
            fee,
        })
        .pack();

    let mut accounts = Vec::with_capacity(4 + signers.len());
    accounts.push(AccountMeta::new(*source, false));
    accounts.push(AccountMeta::new_readonly(*mint, false));
    accounts.push(AccountMeta::new(*destination, false));
    accounts.push(AccountMeta::new_readonly(*authority, signers.is_empty()));
    for signer in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer, true));
    }

    Instruction {
        program_id: id(),
        accounts,
        data,
    }
}

/// Creates a `WithdrawWithheldTokensFromMint` instruction
pub fn withdraw_withheld_tokens_from_mint(
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
) -> Instruction {
    let mut accounts = Vec::with_capacity(3 + signers.len());
    accounts.push(AccountMeta::new(*mint, false));
    accounts.push(AccountMeta::new(*destination, false));
    accounts.push(AccountMeta::new_readonly(*authority, signers.is_empty()));
    for signer in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer, true));
    }

    Instruction {
        program_id: id(),
        accounts,
        data: TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::WithdrawWithheldTokensFromMint,
        )
        .pack(),
    }
}

/// Creates a `WithdrawWithheldTokensFromAccounts` instruction
pub fn withdraw_withheld_tokens_from_accounts(
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
    sources: &[&Pubkey],
) -> Instruction {
    let mut accounts = Vec::with_capacity(3 + signers.len() + sources.len());
    accounts.push(AccountMeta::new_readonly(*mint, false));
    accounts.push(AccountMeta::new(*destination, false));
    accounts.push(AccountMeta::new_readonly(*authority, signers.is_empty()));
    for signer in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer, true));
    }
    for source in sources.iter() {
        accounts.push(AccountMeta::new(**source, false));
    }

    Instruction {
        program_id: id(),
        accounts,
        data: TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::WithdrawWithheldTokensFromAccounts,
        )
        .pack(),
    }
}

/// Creates a `HarvestWithheldTokensToMint` instruction
pub fn harvest_withheld_tokens_to_mint(mint: &Pubkey, sources: &[&Pubkey]) -> Instruction {
    let mut accounts = Vec::with_capacity(1 + sources.len());
    accounts.push(AccountMeta::new(*mint, false));
    for source in sources.iter() {
        accounts.push(AccountMeta::new(**source, false));
    }
    Instruction {
        program_id: id(),
        accounts,
        data: TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::HarvestWithheldTokensToMint,
        )
        .pack(),
    }
}

/// Creates a `SetTransferFee` instruction
pub fn set_transfer_fee(
    mint: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Instruction {
    let mut accounts = Vec::with_capacity(2 + signers.len());
    accounts.push(AccountMeta::new(*mint, false));
    accounts.push(AccountMeta::new_readonly(*authority, signers.is_empty()));
    for signer in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer, true));
    }

    Instruction {
        program_id: id(),
        accounts,
        data: TokenInstruction::TransferFeeExtension(TransferFeeInstruction::SetTransferFee {
            transfer_fee_basis_points,
            maximum_fee,
        })
        .pack(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instruction_packing() {
        let check = TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::InitializeTransferFeeConfig {
                transfer_fee_config_authority: COption::Some(Pubkey::new(&[11u8; 32])),
                withdraw_withheld_authority: COption::None,
                transfer_fee_basis_points: 111,
                maximum_fee: u64::MAX,
            },
        );
        let packed = check.pack();
        let mut expect = vec![23u8, 0, 1];
        expect.extend_from_slice(&[11u8; 32]);
        expect.extend_from_slice(&[0]);
        expect.extend_from_slice(&111u16.to_le_bytes());
        expect.extend_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::TransferCheckedWithFee {
                amount: 24,
                decimals: 24,
                fee: 23,
            },
        );
        let packed = check.pack();
        let mut expect = vec![23u8, 1];
        expect.extend_from_slice(&24u64.to_le_bytes());
        expect.extend_from_slice(&[24u8]);
        expect.extend_from_slice(&23u64.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::WithdrawWithheldTokensFromMint,
        );
        let packed = check.pack();
        let expect = [23u8, 2];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::WithdrawWithheldTokensFromAccounts,
        );
        let packed = check.pack();
        let expect = [23u8, 3];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::TransferFeeExtension(
            TransferFeeInstruction::HarvestWithheldTokensToMint,
        );
        let packed = check.pack();
        let expect = [23u8, 4];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check =
            TokenInstruction::TransferFeeExtension(TransferFeeInstruction::SetTransferFee {
                transfer_fee_basis_points: u16::MAX,
                maximum_fee: u64::MAX,
            });
        let packed = check.pack();
        let mut expect = vec![23u8, 5];
        expect.extend_from_slice(&u16::MAX.to_le_bytes());
        expect.extend_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
