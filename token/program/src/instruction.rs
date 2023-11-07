//! Instruction types

use {
    crate::{check_program_account, error::TokenError},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
        sysvar,
    },
    std::{convert::TryInto, mem::size_of},
};

/// Minimum number of multisignature signers (min N)
pub const MIN_SIGNERS: usize = 1;
/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;
/// Serialized length of a u64, for unpacking
const U64_BYTES: usize = 8;

/// Instructions supported by the token program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenInstruction<'a> {
    /// Initializes a new mint and optionally deposits all the newly minted
    /// tokens in an account.
    ///
    /// The `InitializeMint` instruction requires no signers and MUST be
    /// included within the same Transaction as the system program's
    /// `CreateAccount` instruction that creates the account being initialized.
    /// Otherwise another party can acquire ownership of the uninitialized
    /// account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to initialize.
    ///   1. `[]` Rent sysvar
    InitializeMint {
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        /// The authority/multisignature to mint tokens.
        mint_authority: Pubkey,
        /// The freeze authority/multisignature of the mint.
        freeze_authority: COption<Pubkey>,
    },
    /// Initializes a new account to hold tokens.  If this account is associated
    /// with the native mint then the token balance of the initialized account
    /// will be equal to the amount of SOL in the account. If this account is
    /// associated with another mint, that mint must be initialized before this
    /// command can succeed.
    ///
    /// The `InitializeAccount` instruction requires no signers and MUST be
    /// included within the same Transaction as the system program's
    /// `CreateAccount` instruction that creates the account being initialized.
    /// Otherwise another party can acquire ownership of the uninitialized
    /// account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The account to initialize.
    ///   1. `[]` The mint this account will be associated with.
    ///   2. `[]` The new account's owner/multisignature.
    ///   3. `[]` Rent sysvar
    InitializeAccount,
    /// Initializes a multisignature account with N provided signers.
    ///
    /// Multisignature accounts can used in place of any single owner/delegate
    /// accounts in any token instruction that require an owner/delegate to be
    /// present.  The variant field represents the number of signers (M)
    /// required to validate this multisignature account.
    ///
    /// The `InitializeMultisig` instruction requires no signers and MUST be
    /// included within the same Transaction as the system program's
    /// `CreateAccount` instruction that creates the account being initialized.
    /// Otherwise another party can acquire ownership of the uninitialized
    /// account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The multisignature account to initialize.
    ///   1. `[]` Rent sysvar
    ///   2. ..2+N. `[]` The signer accounts, must equal to N where 1 <= N <=
    ///      11.
    InitializeMultisig {
        /// The number of signers (M) required to validate this multisignature
        /// account.
        m: u8,
    },
    /// Transfers tokens from one account to another either directly or via a
    /// delegate.  If this account is associated with the native mint then equal
    /// amounts of SOL and Tokens will be transferred to the destination
    /// account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The source account.
    ///   1. `[writable]` The destination account.
    ///   2. `[signer]` The source account's owner/delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The source account.
    ///   1. `[writable]` The destination account.
    ///   2. `[]` The source account's multisignature owner/delegate.
    ///   3. ..3+M `[signer]` M signer accounts.
    Transfer {
        /// The amount of tokens to transfer.
        amount: u64,
    },
    /// Approves a delegate.  A delegate is given the authority over tokens on
    /// behalf of the source account's owner.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The source account.
    ///   1. `[]` The delegate.
    ///   2. `[signer]` The source account owner.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The source account.
    ///   1. `[]` The delegate.
    ///   2. `[]` The source account's multisignature owner.
    ///   3. ..3+M `[signer]` M signer accounts
    Approve {
        /// The amount of tokens the delegate is approved for.
        amount: u64,
    },
    /// Revokes the delegate's authority.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The source account.
    ///   1. `[signer]` The source account owner.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The source account.
    ///   1. `[]` The source account's multisignature owner.
    ///   2. ..2+M `[signer]` M signer accounts
    Revoke,
    /// Sets a new authority of a mint or account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint or account to change the authority of.
    ///   1. `[signer]` The current authority of the mint or account.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint or account to change the authority of.
    ///   1. `[]` The mint's or account's current multisignature authority.
    ///   2. ..2+M `[signer]` M signer accounts
    SetAuthority {
        /// The type of authority to update.
        authority_type: AuthorityType,
        /// The new authority
        new_authority: COption<Pubkey>,
    },
    /// Mints new tokens to an account.  The native mint does not support
    /// minting.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[writable]` The account to mint tokens to.
    ///   2. `[signer]` The mint's minting authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[writable]` The account to mint tokens to.
    ///   2. `[]` The mint's multisignature mint-tokens authority.
    ///   3. ..3+M `[signer]` M signer accounts.
    MintTo {
        /// The amount of new tokens to mint.
        amount: u64,
    },
    /// Burns tokens by removing them from an account.  `Burn` does not support
    /// accounts associated with the native mint, use `CloseAccount` instead.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The account to burn from.
    ///   1. `[writable]` The token mint.
    ///   2. `[signer]` The account's owner/delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The account to burn from.
    ///   1. `[writable]` The token mint.
    ///   2. `[]` The account's multisignature owner/delegate.
    ///   3. ..3+M `[signer]` M signer accounts.
    Burn {
        /// The amount of tokens to burn.
        amount: u64,
    },
    /// Close an account by transferring all its SOL to the destination account.
    /// Non-native accounts may only be closed if its token amount is zero.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The account to close.
    ///   1. `[writable]` The destination account.
    ///   2. `[signer]` The account's owner.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The account to close.
    ///   1. `[writable]` The destination account.
    ///   2. `[]` The account's multisignature owner.
    ///   3. ..3+M `[signer]` M signer accounts.
    CloseAccount,
    /// Freeze an Initialized account using the Mint's freeze_authority (if
    /// set).
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The account to freeze.
    ///   1. `[]` The token mint.
    ///   2. `[signer]` The mint freeze authority.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The account to freeze.
    ///   1. `[]` The token mint.
    ///   2. `[]` The mint's multisignature freeze authority.
    ///   3. ..3+M `[signer]` M signer accounts.
    FreezeAccount,
    /// Thaw a Frozen account using the Mint's freeze_authority (if set).
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The account to freeze.
    ///   1. `[]` The token mint.
    ///   2. `[signer]` The mint freeze authority.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The account to freeze.
    ///   1. `[]` The token mint.
    ///   2. `[]` The mint's multisignature freeze authority.
    ///   3. ..3+M `[signer]` M signer accounts.
    ThawAccount,

    /// Transfers tokens from one account to another either directly or via a
    /// delegate.  If this account is associated with the native mint then equal
    /// amounts of SOL and Tokens will be transferred to the destination
    /// account.
    ///
    /// This instruction differs from Transfer in that the token mint and
    /// decimals value is checked by the caller.  This may be useful when
    /// creating transactions offline or within a hardware wallet.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The source account.
    ///   1. `[]` The token mint.
    ///   2. `[writable]` The destination account.
    ///   3. `[signer]` The source account's owner/delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The source account.
    ///   1. `[]` The token mint.
    ///   2. `[writable]` The destination account.
    ///   3. `[]` The source account's multisignature owner/delegate.
    ///   4. ..4+M `[signer]` M signer accounts.
    TransferChecked {
        /// The amount of tokens to transfer.
        amount: u64,
        /// Expected number of base 10 digits to the right of the decimal place.
        decimals: u8,
    },
    /// Approves a delegate.  A delegate is given the authority over tokens on
    /// behalf of the source account's owner.
    ///
    /// This instruction differs from Approve in that the token mint and
    /// decimals value is checked by the caller.  This may be useful when
    /// creating transactions offline or within a hardware wallet.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The source account.
    ///   1. `[]` The token mint.
    ///   2. `[]` The delegate.
    ///   3. `[signer]` The source account owner.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The source account.
    ///   1. `[]` The token mint.
    ///   2. `[]` The delegate.
    ///   3. `[]` The source account's multisignature owner.
    ///   4. ..4+M `[signer]` M signer accounts
    ApproveChecked {
        /// The amount of tokens the delegate is approved for.
        amount: u64,
        /// Expected number of base 10 digits to the right of the decimal place.
        decimals: u8,
    },
    /// Mints new tokens to an account.  The native mint does not support
    /// minting.
    ///
    /// This instruction differs from MintTo in that the decimals value is
    /// checked by the caller.  This may be useful when creating transactions
    /// offline or within a hardware wallet.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[writable]` The account to mint tokens to.
    ///   2. `[signer]` The mint's minting authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[writable]` The account to mint tokens to.
    ///   2. `[]` The mint's multisignature mint-tokens authority.
    ///   3. ..3+M `[signer]` M signer accounts.
    MintToChecked {
        /// The amount of new tokens to mint.
        amount: u64,
        /// Expected number of base 10 digits to the right of the decimal place.
        decimals: u8,
    },
    /// Burns tokens by removing them from an account.  `BurnChecked` does not
    /// support accounts associated with the native mint, use `CloseAccount`
    /// instead.
    ///
    /// This instruction differs from Burn in that the decimals value is checked
    /// by the caller. This may be useful when creating transactions offline or
    /// within a hardware wallet.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The account to burn from.
    ///   1. `[writable]` The token mint.
    ///   2. `[signer]` The account's owner/delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The account to burn from.
    ///   1. `[writable]` The token mint.
    ///   2. `[]` The account's multisignature owner/delegate.
    ///   3. ..3+M `[signer]` M signer accounts.
    BurnChecked {
        /// The amount of tokens to burn.
        amount: u64,
        /// Expected number of base 10 digits to the right of the decimal place.
        decimals: u8,
    },
    /// Like InitializeAccount, but the owner pubkey is passed via instruction
    /// data rather than the accounts list. This variant may be preferable
    /// when using Cross Program Invocation from an instruction that does
    /// not need the owner's `AccountInfo` otherwise.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The account to initialize.
    ///   1. `[]` The mint this account will be associated with.
    ///   3. `[]` Rent sysvar
    InitializeAccount2 {
        /// The new account's owner/multisignature.
        owner: Pubkey,
    },
    /// Given a wrapped / native token account (a token account containing SOL)
    /// updates its amount field based on the account's underlying `lamports`.
    /// This is useful if a non-wrapped SOL account uses
    /// `system_instruction::transfer` to move lamports to a wrapped token
    /// account, and needs to have its token `amount` field updated.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The native token account to sync with its underlying
    ///      lamports.
    SyncNative,
    /// Like InitializeAccount2, but does not require the Rent sysvar to be
    /// provided
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The account to initialize.
    ///   1. `[]` The mint this account will be associated with.
    InitializeAccount3 {
        /// The new account's owner/multisignature.
        owner: Pubkey,
    },
    /// Like InitializeMultisig, but does not require the Rent sysvar to be
    /// provided
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The multisignature account to initialize.
    ///   1. ..1+N. `[]` The signer accounts, must equal to N where 1 <= N <=
    ///      11.
    InitializeMultisig2 {
        /// The number of signers (M) required to validate this multisignature
        /// account.
        m: u8,
    },
    /// Like [`InitializeMint`], but does not require the Rent sysvar to be
    /// provided
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to initialize.
    InitializeMint2 {
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        /// The authority/multisignature to mint tokens.
        mint_authority: Pubkey,
        /// The freeze authority/multisignature of the mint.
        freeze_authority: COption<Pubkey>,
    },
    /// Gets the required size of an account for the given mint as a
    /// little-endian `u64`.
    ///
    /// Return data can be fetched using `sol_get_return_data` and deserializing
    /// the return data as a little-endian `u64`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` The mint to calculate for
    GetAccountDataSize, // typically, there's also data, but this program ignores it
    /// Initialize the Immutable Owner extension for the given token account
    ///
    /// Fails if the account has already been initialized, so must be called
    /// before `InitializeAccount`.
    ///
    /// No-ops in this version of the program, but is included for compatibility
    /// with the Associated Token Account program.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The account to initialize.
    ///
    /// Data expected by this instruction:
    ///   None
    InitializeImmutableOwner,
    /// Convert an Amount of tokens to a UiAmount `string`, using the given
    /// mint. In this version of the program, the mint can only specify the
    /// number of decimals.
    ///
    /// Fails on an invalid mint.
    ///
    /// Return data can be fetched using `sol_get_return_data` and deserialized
    /// with `String::from_utf8`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` The mint to calculate for
    AmountToUiAmount {
        /// The amount of tokens to reformat.
        amount: u64,
    },
    /// Convert a UiAmount of tokens to a little-endian `u64` raw Amount, using
    /// the given mint. In this version of the program, the mint can only
    /// specify the number of decimals.
    ///
    /// Return data can be fetched using `sol_get_return_data` and deserializing
    /// the return data as a little-endian `u64`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` The mint to calculate for
    UiAmountToAmount {
        /// The ui_amount of tokens to reformat.
        ui_amount: &'a str,
    },
    // Any new variants also need to be added to program-2022 `TokenInstruction`, so that the
    // latter remains a superset of this instruction set. New variants also need to be added to
    // token/js/src/instructions/types.ts to maintain @solana/spl-token compatibility
}
impl<'a> TokenInstruction<'a> {
    /// Unpacks a byte buffer into a
    /// [TokenInstruction](enum.TokenInstruction.html).
    pub fn unpack(input: &'a [u8]) -> Result<Self, ProgramError> {
        use TokenError::InvalidInstruction;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let (&decimals, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let (mint_authority, rest) = Self::unpack_pubkey(rest)?;
                let (freeze_authority, _rest) = Self::unpack_pubkey_option(rest)?;
                Self::InitializeMint {
                    mint_authority,
                    freeze_authority,
                    decimals,
                }
            }
            1 => Self::InitializeAccount,
            2 => {
                let &m = rest.first().ok_or(InvalidInstruction)?;
                Self::InitializeMultisig { m }
            }
            3 | 4 | 7 | 8 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                match tag {
                    3 => Self::Transfer { amount },
                    4 => Self::Approve { amount },
                    7 => Self::MintTo { amount },
                    8 => Self::Burn { amount },
                    _ => unreachable!(),
                }
            }
            5 => Self::Revoke,
            6 => {
                let (authority_type, rest) = rest
                    .split_first()
                    .ok_or_else(|| ProgramError::from(InvalidInstruction))
                    .and_then(|(&t, rest)| Ok((AuthorityType::from(t)?, rest)))?;
                let (new_authority, _rest) = Self::unpack_pubkey_option(rest)?;

                Self::SetAuthority {
                    authority_type,
                    new_authority,
                }
            }
            9 => Self::CloseAccount,
            10 => Self::FreezeAccount,
            11 => Self::ThawAccount,
            12 => {
                let (amount, decimals, _rest) = Self::unpack_amount_decimals(rest)?;
                Self::TransferChecked { amount, decimals }
            }
            13 => {
                let (amount, decimals, _rest) = Self::unpack_amount_decimals(rest)?;
                Self::ApproveChecked { amount, decimals }
            }
            14 => {
                let (amount, decimals, _rest) = Self::unpack_amount_decimals(rest)?;
                Self::MintToChecked { amount, decimals }
            }
            15 => {
                let (amount, decimals, _rest) = Self::unpack_amount_decimals(rest)?;
                Self::BurnChecked { amount, decimals }
            }
            16 => {
                let (owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::InitializeAccount2 { owner }
            }
            17 => Self::SyncNative,
            18 => {
                let (owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::InitializeAccount3 { owner }
            }
            19 => {
                let &m = rest.first().ok_or(InvalidInstruction)?;
                Self::InitializeMultisig2 { m }
            }
            20 => {
                let (&decimals, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let (mint_authority, rest) = Self::unpack_pubkey(rest)?;
                let (freeze_authority, _rest) = Self::unpack_pubkey_option(rest)?;
                Self::InitializeMint2 {
                    mint_authority,
                    freeze_authority,
                    decimals,
                }
            }
            21 => Self::GetAccountDataSize,
            22 => Self::InitializeImmutableOwner,
            23 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::AmountToUiAmount { amount }
            }
            24 => {
                let ui_amount = std::str::from_utf8(rest).map_err(|_| InvalidInstruction)?;
                Self::UiAmountToAmount { ui_amount }
            }
            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }

    /// Packs a [TokenInstruction](enum.TokenInstruction.html) into a byte
    /// buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            &Self::InitializeMint {
                ref mint_authority,
                ref freeze_authority,
                decimals,
            } => {
                buf.push(0);
                buf.push(decimals);
                buf.extend_from_slice(mint_authority.as_ref());
                Self::pack_pubkey_option(freeze_authority, &mut buf);
            }
            Self::InitializeAccount => buf.push(1),
            &Self::InitializeMultisig { m } => {
                buf.push(2);
                buf.push(m);
            }
            &Self::Transfer { amount } => {
                buf.push(3);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::Approve { amount } => {
                buf.push(4);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::MintTo { amount } => {
                buf.push(7);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::Burn { amount } => {
                buf.push(8);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::Revoke => buf.push(5),
            Self::SetAuthority {
                authority_type,
                ref new_authority,
            } => {
                buf.push(6);
                buf.push(authority_type.into());
                Self::pack_pubkey_option(new_authority, &mut buf);
            }
            Self::CloseAccount => buf.push(9),
            Self::FreezeAccount => buf.push(10),
            Self::ThawAccount => buf.push(11),
            &Self::TransferChecked { amount, decimals } => {
                buf.push(12);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::ApproveChecked { amount, decimals } => {
                buf.push(13);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::MintToChecked { amount, decimals } => {
                buf.push(14);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::BurnChecked { amount, decimals } => {
                buf.push(15);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::InitializeAccount2 { owner } => {
                buf.push(16);
                buf.extend_from_slice(owner.as_ref());
            }
            &Self::SyncNative => {
                buf.push(17);
            }
            &Self::InitializeAccount3 { owner } => {
                buf.push(18);
                buf.extend_from_slice(owner.as_ref());
            }
            &Self::InitializeMultisig2 { m } => {
                buf.push(19);
                buf.push(m);
            }
            &Self::InitializeMint2 {
                ref mint_authority,
                ref freeze_authority,
                decimals,
            } => {
                buf.push(20);
                buf.push(decimals);
                buf.extend_from_slice(mint_authority.as_ref());
                Self::pack_pubkey_option(freeze_authority, &mut buf);
            }
            &Self::GetAccountDataSize => {
                buf.push(21);
            }
            &Self::InitializeImmutableOwner => {
                buf.push(22);
            }
            &Self::AmountToUiAmount { amount } => {
                buf.push(23);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::UiAmountToAmount { ui_amount } => {
                buf.push(24);
                buf.extend_from_slice(ui_amount.as_bytes());
            }
        };
        buf
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::try_from(key).map_err(|_| TokenError::InvalidInstruction)?;
            Ok((pk, rest))
        } else {
            Err(TokenError::InvalidInstruction.into())
        }
    }

    fn unpack_pubkey_option(input: &[u8]) -> Result<(COption<Pubkey>, &[u8]), ProgramError> {
        match input.split_first() {
            Option::Some((&0, rest)) => Ok((COption::None, rest)),
            Option::Some((&1, rest)) if rest.len() >= 32 => {
                let (key, rest) = rest.split_at(32);
                let pk = Pubkey::try_from(key).map_err(|_| TokenError::InvalidInstruction)?;
                Ok((COption::Some(pk), rest))
            }
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }

    fn pack_pubkey_option(value: &COption<Pubkey>, buf: &mut Vec<u8>) {
        match *value {
            COption::Some(ref key) => {
                buf.push(1);
                buf.extend_from_slice(&key.to_bytes());
            }
            COption::None => buf.push(0),
        }
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        let value = input
            .get(..U64_BYTES)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(TokenError::InvalidInstruction)?;
        Ok((value, &input[U64_BYTES..]))
    }

    fn unpack_amount_decimals(input: &[u8]) -> Result<(u64, u8, &[u8]), ProgramError> {
        let (amount, rest) = Self::unpack_u64(input)?;
        let (&decimals, rest) = rest.split_first().ok_or(TokenError::InvalidInstruction)?;
        Ok((amount, decimals, rest))
    }
}

/// Specifies the authority type for SetAuthority instructions
#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum AuthorityType {
    /// Authority to mint new tokens
    MintTokens,
    /// Authority to freeze any account associated with the Mint
    FreezeAccount,
    /// Owner of a given token account
    AccountOwner,
    /// Authority to close a token account
    CloseAccount,
}

impl AuthorityType {
    fn into(&self) -> u8 {
        match self {
            AuthorityType::MintTokens => 0,
            AuthorityType::FreezeAccount => 1,
            AuthorityType::AccountOwner => 2,
            AuthorityType::CloseAccount => 3,
        }
    }

    fn from(index: u8) -> Result<Self, ProgramError> {
        match index {
            0 => Ok(AuthorityType::MintTokens),
            1 => Ok(AuthorityType::FreezeAccount),
            2 => Ok(AuthorityType::AccountOwner),
            3 => Ok(AuthorityType::CloseAccount),
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }
}

/// Creates a `InitializeMint` instruction.
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    mint_authority_pubkey: &Pubkey,
    freeze_authority_pubkey: Option<&Pubkey>,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let freeze_authority = freeze_authority_pubkey.cloned().into();
    let data = TokenInstruction::InitializeMint {
        mint_authority: *mint_authority_pubkey,
        freeze_authority,
        decimals,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*mint_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeMint2` instruction.
pub fn initialize_mint2(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    mint_authority_pubkey: &Pubkey,
    freeze_authority_pubkey: Option<&Pubkey>,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let freeze_authority = freeze_authority_pubkey.cloned().into();
    let data = TokenInstruction::InitializeMint2 {
        mint_authority: *mint_authority_pubkey,
        freeze_authority,
        decimals,
    }
    .pack();

    let accounts = vec![AccountMeta::new(*mint_pubkey, false)];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeAccount` instruction.
pub fn initialize_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::InitializeAccount.pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeAccount2` instruction.
pub fn initialize_account2(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::InitializeAccount2 {
        owner: *owner_pubkey,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeAccount3` instruction.
pub fn initialize_account3(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::InitializeAccount3 {
        owner: *owner_pubkey,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeMultisig` instruction.
pub fn initialize_multisig(
    token_program_id: &Pubkey,
    multisig_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    m: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    if !is_valid_signer_index(m as usize)
        || !is_valid_signer_index(signer_pubkeys.len())
        || m as usize > signer_pubkeys.len()
    {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let data = TokenInstruction::InitializeMultisig { m }.pack();

    let mut accounts = Vec::with_capacity(1 + 1 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*multisig_pubkey, false));
    accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, false));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeMultisig2` instruction.
pub fn initialize_multisig2(
    token_program_id: &Pubkey,
    multisig_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    m: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    if !is_valid_signer_index(m as usize)
        || !is_valid_signer_index(signer_pubkeys.len())
        || m as usize > signer_pubkeys.len()
    {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let data = TokenInstruction::InitializeMultisig2 { m }.pack();

    let mut accounts = Vec::with_capacity(1 + 1 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*multisig_pubkey, false));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, false));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `Transfer` instruction.
pub fn transfer(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::Transfer { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `Approve` instruction.
pub fn approve(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    delegate_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::Approve { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*delegate_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `Revoke` instruction.
pub fn revoke(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::Revoke.pack();

    let mut accounts = Vec::with_capacity(2 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `SetAuthority` instruction.
pub fn set_authority(
    token_program_id: &Pubkey,
    owned_pubkey: &Pubkey,
    new_authority_pubkey: Option<&Pubkey>,
    authority_type: AuthorityType,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let new_authority = new_authority_pubkey.cloned().into();
    let data = TokenInstruction::SetAuthority {
        authority_type,
        new_authority,
    }
    .pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*owned_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `MintTo` instruction.
pub fn mint_to(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::MintTo { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `Burn` instruction.
pub fn burn(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::Burn { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `CloseAccount` instruction.
pub fn close_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::CloseAccount.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `FreezeAccount` instruction.
pub fn freeze_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::FreezeAccount.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `ThawAccount` instruction.
pub fn thaw_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::ThawAccount.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `TransferChecked` instruction.
#[allow(clippy::too_many_arguments)]
pub fn transfer_checked(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::TransferChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(4 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `ApproveChecked` instruction.
#[allow(clippy::too_many_arguments)]
pub fn approve_checked(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    delegate_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::ApproveChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(4 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*delegate_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `MintToChecked` instruction.
pub fn mint_to_checked(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::MintToChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `BurnChecked` instruction.
pub fn burn_checked(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let data = TokenInstruction::BurnChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `SyncNative` instruction
pub fn sync_native(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new(*account_pubkey, false)],
        data: TokenInstruction::SyncNative.pack(),
    })
}

/// Creates a `GetAccountDataSize` instruction
pub fn get_account_data_size(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new_readonly(*mint_pubkey, false)],
        data: TokenInstruction::GetAccountDataSize.pack(),
    })
}

/// Creates a `InitializeImmutableOwner` instruction
pub fn initialize_immutable_owner(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new(*account_pubkey, false)],
        data: TokenInstruction::InitializeImmutableOwner.pack(),
    })
}

/// Creates an `AmountToUiAmount` instruction
pub fn amount_to_ui_amount(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new_readonly(*mint_pubkey, false)],
        data: TokenInstruction::AmountToUiAmount { amount }.pack(),
    })
}

/// Creates a `UiAmountToAmount` instruction
pub fn ui_amount_to_amount(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    ui_amount: &str,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new_readonly(*mint_pubkey, false)],
        data: TokenInstruction::UiAmountToAmount { ui_amount }.pack(),
    })
}

/// Utility function that checks index is between MIN_SIGNERS and MAX_SIGNERS
pub fn is_valid_signer_index(index: usize) -> bool {
    (MIN_SIGNERS..=MAX_SIGNERS).contains(&index)
}

#[cfg(test)]
mod test {
    use {super::*, proptest::prelude::*};

    #[test]
    fn test_instruction_packing() {
        let check = TokenInstruction::InitializeMint {
            decimals: 2,
            mint_authority: Pubkey::new_from_array([1u8; 32]),
            freeze_authority: COption::None,
        };
        let packed = check.pack();
        let mut expect = Vec::from([0u8, 2]);
        expect.extend_from_slice(&[1u8; 32]);
        expect.extend_from_slice(&[0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeMint {
            decimals: 2,
            mint_authority: Pubkey::new_from_array([2u8; 32]),
            freeze_authority: COption::Some(Pubkey::new_from_array([3u8; 32])),
        };
        let packed = check.pack();
        let mut expect = vec![0u8, 2];
        expect.extend_from_slice(&[2u8; 32]);
        expect.extend_from_slice(&[1]);
        expect.extend_from_slice(&[3u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeAccount;
        let packed = check.pack();
        let expect = Vec::from([1u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeMultisig { m: 1 };
        let packed = check.pack();
        let expect = Vec::from([2u8, 1]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Transfer { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([3u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Approve { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([4u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Revoke;
        let packed = check.pack();
        let expect = Vec::from([5u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::SetAuthority {
            authority_type: AuthorityType::FreezeAccount,
            new_authority: COption::Some(Pubkey::new_from_array([4u8; 32])),
        };
        let packed = check.pack();
        let mut expect = Vec::from([6u8, 1]);
        expect.extend_from_slice(&[1]);
        expect.extend_from_slice(&[4u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::MintTo { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([7u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Burn { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([8u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::CloseAccount;
        let packed = check.pack();
        let expect = Vec::from([9u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::FreezeAccount;
        let packed = check.pack();
        let expect = Vec::from([10u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::ThawAccount;
        let packed = check.pack();
        let expect = Vec::from([11u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::TransferChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([12u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::ApproveChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([13u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::MintToChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([14u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::BurnChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([15u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeAccount2 {
            owner: Pubkey::new_from_array([2u8; 32]),
        };
        let packed = check.pack();
        let mut expect = vec![16u8];
        expect.extend_from_slice(&[2u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::SyncNative;
        let packed = check.pack();
        let expect = vec![17u8];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeAccount3 {
            owner: Pubkey::new_from_array([2u8; 32]),
        };
        let packed = check.pack();
        let mut expect = vec![18u8];
        expect.extend_from_slice(&[2u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeMultisig2 { m: 1 };
        let packed = check.pack();
        let expect = Vec::from([19u8, 1]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeMint2 {
            decimals: 2,
            mint_authority: Pubkey::new_from_array([1u8; 32]),
            freeze_authority: COption::None,
        };
        let packed = check.pack();
        let mut expect = Vec::from([20u8, 2]);
        expect.extend_from_slice(&[1u8; 32]);
        expect.extend_from_slice(&[0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeMint2 {
            decimals: 2,
            mint_authority: Pubkey::new_from_array([2u8; 32]),
            freeze_authority: COption::Some(Pubkey::new_from_array([3u8; 32])),
        };
        let packed = check.pack();
        let mut expect = vec![20u8, 2];
        expect.extend_from_slice(&[2u8; 32]);
        expect.extend_from_slice(&[1]);
        expect.extend_from_slice(&[3u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::GetAccountDataSize;
        let packed = check.pack();
        let expect = vec![21u8];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeImmutableOwner;
        let packed = check.pack();
        let expect = vec![22u8];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::AmountToUiAmount { amount: 42 };
        let packed = check.pack();
        let expect = vec![23u8, 42, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::UiAmountToAmount { ui_amount: "0.42" };
        let packed = check.pack();
        let expect = vec![24u8, 48, 46, 52, 50];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn test_instruction_unpack_panic() {
        for i in 0..255u8 {
            for j in 1..10 {
                let mut data = vec![0; j];
                data[0] = i;
                let _no_panic = TokenInstruction::unpack(&data);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]
        #[test]
        fn test_instruction_unpack_proptest(
            data in prop::collection::vec(any::<u8>(), 0..255)
        ) {
            let _no_panic = TokenInstruction::unpack(&data);
        }
    }
}
