//! Instruction types

// Needed to avoid deprecation warning when generating serde implementation for
// TokenInstruction
#![allow(deprecated)]

#[cfg(feature = "serde-traits")]
use {
    crate::serialization::coption_fromstr,
    serde::{Deserialize, Serialize},
    serde_with::{As, DisplayFromStr},
};
use {
    crate::{
        check_program_account, check_spl_token_program_account,
        error::TokenError,
        extension::{transfer_fee::instruction::TransferFeeInstruction, ExtensionType},
    },
    bytemuck::Pod,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        program_option::COption,
        pubkey::{Pubkey, PUBKEY_BYTES},
        system_program, sysvar,
    },
    spl_pod::bytemuck::{pod_from_bytes, pod_get_packed_len},
    std::{
        convert::{TryFrom, TryInto},
        mem::size_of,
    },
};

/// Minimum number of multisignature signers (min N)
pub const MIN_SIGNERS: usize = 1;
/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;
/// Serialized length of a u16, for unpacking
const U16_BYTES: usize = 2;
/// Serialized length of a u64, for unpacking
const U64_BYTES: usize = 8;

/// Instructions supported by the token program.
#[repr(C)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde-traits",
    serde(rename_all_fields = "camelCase", rename_all = "camelCase")
)]
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
    /// All extensions must be initialized before calling this instruction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to initialize.
    ///   1. `[]` Rent sysvar
    InitializeMint {
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        /// The authority/multisignature to mint tokens.
        #[cfg_attr(feature = "serde-traits", serde(with = "As::<DisplayFromStr>"))]
        mint_authority: Pubkey,
        /// The freeze authority/multisignature of the mint.
        #[cfg_attr(feature = "serde-traits", serde(with = "coption_fromstr"))]
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
    /// NOTE This instruction is deprecated in favor of `TransferChecked` or
    /// `TransferCheckedWithFee`
    ///
    /// Transfers tokens from one account to another either directly or via a
    /// delegate.  If this account is associated with the native mint then equal
    /// amounts of SOL and Tokens will be transferred to the destination
    /// account.
    ///
    /// If either account contains an `TransferFeeAmount` extension, this will
    /// fail. Mints with the `TransferFeeConfig` extension are required in
    /// order to assess the fee.
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
    #[deprecated(
        since = "4.0.0",
        note = "please use `TransferChecked` or `TransferCheckedWithFee` instead"
    )]
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
    ///   1. `[signer]` The source account owner or current delegate.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The source account.
    ///   1. `[]` The source account's multisignature owner or current delegate.
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
        #[cfg_attr(feature = "serde-traits", serde(with = "coption_fromstr"))]
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
    /// Accounts with the `TransferFeeAmount` extension may only be closed if
    /// the withheld amount is zero.
    ///
    /// Accounts with the `ConfidentialTransfer` extension may only be closed if
    /// the pending and available balance ciphertexts are empty. Use
    /// `ConfidentialTransferInstruction::ApplyPendingBalance` and
    /// `ConfidentialTransferInstruction::EmptyAccount` to empty these
    /// ciphertexts.
    ///
    /// Accounts with the `ConfidentialTransferFee` extension may only be closed
    /// if the withheld amount ciphertext is empty. Use
    /// `ConfidentialTransferFeeInstruction::HarvestWithheldTokensToMint` to
    /// empty this ciphertext.
    ///
    /// Mints may be closed if they have the `MintCloseAuthority` extension and
    /// their token supply is zero
    ///
    /// Accounts
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
    /// If either account contains an `TransferFeeAmount` extension, the fee is
    /// withheld in the destination account.
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
    ///   2. `[]` Rent sysvar
    InitializeAccount2 {
        /// The new account's owner/multisignature.
        #[cfg_attr(feature = "serde-traits", serde(with = "As::<DisplayFromStr>"))]
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
        #[cfg_attr(feature = "serde-traits", serde(with = "As::<DisplayFromStr>"))]
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
    /// Like InitializeMint, but does not require the Rent sysvar to be provided
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to initialize.
    InitializeMint2 {
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        /// The authority/multisignature to mint tokens.
        #[cfg_attr(feature = "serde-traits", serde(with = "As::<DisplayFromStr>"))]
        mint_authority: Pubkey,
        /// The freeze authority/multisignature of the mint.
        #[cfg_attr(feature = "serde-traits", serde(with = "coption_fromstr"))]
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
    GetAccountDataSize {
        /// Additional extension types to include in the returned account size
        extension_types: Vec<ExtensionType>,
    },
    /// Initialize the Immutable Owner extension for the given token account
    ///
    /// Fails if the account has already been initialized, so must be called
    /// before `InitializeAccount`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The account to initialize.
    ///
    /// Data expected by this instruction:
    ///   None
    InitializeImmutableOwner,
    /// Convert an Amount of tokens to a UiAmount `string`, using the given
    /// mint.
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
        /// The amount of tokens to convert.
        amount: u64,
    },
    /// Convert a UiAmount of tokens to a little-endian `u64` raw Amount, using
    /// the given mint.
    ///
    /// Return data can be fetched using `sol_get_return_data` and deserializing
    /// the return data as a little-endian `u64`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` The mint to calculate for
    UiAmountToAmount {
        /// The ui_amount of tokens to convert.
        ui_amount: &'a str,
    },
    /// Initialize the close account authority on a new mint.
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
    InitializeMintCloseAuthority {
        /// Authority that must sign the `CloseAccount` instruction on a mint
        #[cfg_attr(feature = "serde-traits", serde(with = "coption_fromstr"))]
        close_authority: COption<Pubkey>,
    },
    /// The common instruction prefix for Transfer Fee extension instructions.
    ///
    /// See `extension::transfer_fee::instruction::TransferFeeInstruction` for
    /// further details about the extended instructions that share this
    /// instruction prefix
    TransferFeeExtension(TransferFeeInstruction),
    /// The common instruction prefix for Confidential Transfer extension
    /// instructions.
    ///
    /// See `extension::confidential_transfer::instruction::ConfidentialTransferInstruction` for
    /// further details about the extended instructions that share this
    /// instruction prefix
    ConfidentialTransferExtension,
    /// The common instruction prefix for Default Account State extension
    /// instructions.
    ///
    /// See `extension::default_account_state::instruction::DefaultAccountStateInstruction` for
    /// further details about the extended instructions that share this
    /// instruction prefix
    DefaultAccountStateExtension,
    /// Check to see if a token account is large enough for a list of
    /// ExtensionTypes, and if not, use reallocation to increase the data
    /// size.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The account to reallocate.
    ///   1. `[signer, writable]` The payer account to fund reallocation
    ///   2. `[]` System program for reallocation funding
    ///   3. `[signer]` The account's owner.
    ///
    ///   * Multisignature owner
    ///   0. `[writable]` The account to reallocate.
    ///   1. `[signer, writable]` The payer account to fund reallocation
    ///   2. `[]` System program for reallocation funding
    ///   3. `[]` The account's multisignature owner/delegate.
    ///   4. ..4+M `[signer]` M signer accounts.
    Reallocate {
        /// New extension types to include in the reallocated account
        extension_types: Vec<ExtensionType>,
    },
    /// The common instruction prefix for Memo Transfer account extension
    /// instructions.
    ///
    /// See `extension::memo_transfer::instruction::RequiredMemoTransfersInstruction` for
    /// further details about the extended instructions that share this
    /// instruction prefix
    MemoTransferExtension,
    /// Creates the native mint.
    ///
    /// This instruction only needs to be invoked once after deployment and is
    /// permissionless, Wrapped SOL (`native_mint::id()`) will not be
    /// available until this instruction is successfully executed.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[writable]` The native mint address
    ///   2. `[]` System program for mint account funding
    CreateNativeMint,
    /// Initialize the non transferable extension for the given mint account
    ///
    /// Fails if the account has already been initialized, so must be called
    /// before `InitializeMint`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The mint account to initialize.
    ///
    /// Data expected by this instruction:
    ///   None
    InitializeNonTransferableMint,
    /// The common instruction prefix for Interest Bearing extension
    /// instructions.
    ///
    /// See `extension::interest_bearing_mint::instruction::InterestBearingMintInstruction` for
    /// further details about the extended instructions that share this
    /// instruction prefix
    InterestBearingMintExtension,
    /// The common instruction prefix for CPI Guard account extension
    /// instructions.
    ///
    /// See `extension::cpi_guard::instruction::CpiGuardInstruction` for
    /// further details about the extended instructions that share this
    /// instruction prefix
    CpiGuardExtension,
    /// Initialize the permanent delegate on a new mint.
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
    ///
    /// Data expected by this instruction:
    ///   Pubkey for the permanent delegate
    InitializePermanentDelegate {
        /// Authority that may sign for `Transfer`s and `Burn`s on any account
        #[cfg_attr(feature = "serde-traits", serde(with = "As::<DisplayFromStr>"))]
        delegate: Pubkey,
    },
    /// The common instruction prefix for transfer hook extension instructions.
    ///
    /// See `extension::transfer_hook::instruction::TransferHookInstruction`
    /// for further details about the extended instructions that share this
    /// instruction prefix
    TransferHookExtension,
    /// The common instruction prefix for the confidential transfer fee
    /// extension instructions.
    ///
    /// See `extension::confidential_transfer_fee::instruction::ConfidentialTransferFeeInstruction`
    /// for further details about the extended instructions that share this
    /// instruction prefix
    ConfidentialTransferFeeExtension,
    /// This instruction is to be used to rescue SOLs sent to any TokenProgram
    /// owned account by sending them to any other account, leaving behind only
    /// lamports for rent exemption.
    ///
    /// 0. `[writable]` Source Account owned by the token program
    /// 1. `[writable]` Destination account
    /// 2. `[signer]` Authority
    /// 3. ..2+M `[signer]` M signer accounts.
    WithdrawExcessLamports,
    /// The common instruction prefix for metadata pointer extension
    /// instructions.
    ///
    /// See `extension::metadata_pointer::instruction::MetadataPointerInstruction`
    /// for further details about the extended instructions that share this
    /// instruction prefix
    MetadataPointerExtension,
    /// The common instruction prefix for group pointer extension instructions.
    ///
    /// See `extension::group_pointer::instruction::GroupPointerInstruction`
    /// for further details about the extended instructions that share this
    /// instruction prefix
    GroupPointerExtension,
    /// The common instruction prefix for group member pointer extension
    /// instructions.
    ///
    /// See `extension::group_member_pointer::instruction::GroupMemberPointerInstruction`
    /// for further details about the extended instructions that share this
    /// instruction prefix
    GroupMemberPointerExtension,
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
                    .get(..U64_BYTES)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                match tag {
                    #[allow(deprecated)]
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
            21 => {
                let mut extension_types = vec![];
                for chunk in rest.chunks(size_of::<ExtensionType>()) {
                    extension_types.push(chunk.try_into()?);
                }
                Self::GetAccountDataSize { extension_types }
            }
            22 => Self::InitializeImmutableOwner,
            23 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::AmountToUiAmount { amount }
            }
            24 => {
                let ui_amount = std::str::from_utf8(rest).map_err(|_| InvalidInstruction)?;
                Self::UiAmountToAmount { ui_amount }
            }
            25 => {
                let (close_authority, _rest) = Self::unpack_pubkey_option(rest)?;
                Self::InitializeMintCloseAuthority { close_authority }
            }
            26 => {
                let (instruction, _rest) = TransferFeeInstruction::unpack(rest)?;
                Self::TransferFeeExtension(instruction)
            }
            27 => Self::ConfidentialTransferExtension,
            28 => Self::DefaultAccountStateExtension,
            29 => {
                let mut extension_types = vec![];
                for chunk in rest.chunks(size_of::<ExtensionType>()) {
                    extension_types.push(chunk.try_into()?);
                }
                Self::Reallocate { extension_types }
            }
            30 => Self::MemoTransferExtension,
            31 => Self::CreateNativeMint,
            32 => Self::InitializeNonTransferableMint,
            33 => Self::InterestBearingMintExtension,
            34 => Self::CpiGuardExtension,
            35 => {
                let (delegate, _rest) = Self::unpack_pubkey(rest)?;
                Self::InitializePermanentDelegate { delegate }
            }
            36 => Self::TransferHookExtension,
            37 => Self::ConfidentialTransferFeeExtension,
            38 => Self::WithdrawExcessLamports,
            39 => Self::MetadataPointerExtension,
            40 => Self::GroupPointerExtension,
            41 => Self::GroupMemberPointerExtension,
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
            #[allow(deprecated)]
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
            Self::GetAccountDataSize { extension_types } => {
                buf.push(21);
                for extension_type in extension_types {
                    buf.extend_from_slice(&<[u8; 2]>::from(*extension_type));
                }
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
            Self::InitializeMintCloseAuthority { close_authority } => {
                buf.push(25);
                Self::pack_pubkey_option(close_authority, &mut buf);
            }
            Self::TransferFeeExtension(instruction) => {
                buf.push(26);
                TransferFeeInstruction::pack(instruction, &mut buf);
            }
            &Self::ConfidentialTransferExtension => {
                buf.push(27);
            }
            &Self::DefaultAccountStateExtension => {
                buf.push(28);
            }
            Self::Reallocate { extension_types } => {
                buf.push(29);
                for extension_type in extension_types {
                    buf.extend_from_slice(&<[u8; 2]>::from(*extension_type));
                }
            }
            &Self::MemoTransferExtension => {
                buf.push(30);
            }
            &Self::CreateNativeMint => {
                buf.push(31);
            }
            &Self::InitializeNonTransferableMint => {
                buf.push(32);
            }
            &Self::InterestBearingMintExtension => {
                buf.push(33);
            }
            &Self::CpiGuardExtension => {
                buf.push(34);
            }
            Self::InitializePermanentDelegate { delegate } => {
                buf.push(35);
                buf.extend_from_slice(delegate.as_ref());
            }
            &Self::TransferHookExtension => {
                buf.push(36);
            }
            &Self::ConfidentialTransferFeeExtension => {
                buf.push(37);
            }
            &Self::WithdrawExcessLamports => {
                buf.push(38);
            }
            &Self::MetadataPointerExtension => {
                buf.push(39);
            }
            &Self::GroupPointerExtension => {
                buf.push(40);
            }
            &Self::GroupMemberPointerExtension => {
                buf.push(41);
            }
        };
        buf
    }

    pub(crate) fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        let pk = input
            .get(..PUBKEY_BYTES)
            .and_then(|x| Pubkey::try_from(x).ok())
            .ok_or(TokenError::InvalidInstruction)?;
        Ok((pk, &input[PUBKEY_BYTES..]))
    }

    pub(crate) fn unpack_pubkey_option(
        input: &[u8],
    ) -> Result<(COption<Pubkey>, &[u8]), ProgramError> {
        match input.split_first() {
            Option::Some((&0, rest)) => Ok((COption::None, rest)),
            Option::Some((&1, rest)) => {
                let (pk, rest) = Self::unpack_pubkey(rest)?;
                Ok((COption::Some(pk), rest))
            }
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }

    pub(crate) fn pack_pubkey_option(value: &COption<Pubkey>, buf: &mut Vec<u8>) {
        match *value {
            COption::Some(ref key) => {
                buf.push(1);
                buf.extend_from_slice(&key.to_bytes());
            }
            COption::None => buf.push(0),
        }
    }

    pub(crate) fn unpack_u16(input: &[u8]) -> Result<(u16, &[u8]), ProgramError> {
        let value = input
            .get(..U16_BYTES)
            .and_then(|slice| slice.try_into().ok())
            .map(u16::from_le_bytes)
            .ok_or(TokenError::InvalidInstruction)?;
        Ok((value, &input[U16_BYTES..]))
    }

    pub(crate) fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        let value = input
            .get(..U64_BYTES)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(TokenError::InvalidInstruction)?;
        Ok((value, &input[U64_BYTES..]))
    }

    pub(crate) fn unpack_amount_decimals(input: &[u8]) -> Result<(u64, u8, &[u8]), ProgramError> {
        let (amount, rest) = Self::unpack_u64(input)?;
        let (&decimals, rest) = rest.split_first().ok_or(TokenError::InvalidInstruction)?;
        Ok((amount, decimals, rest))
    }
}

/// Specifies the authority type for SetAuthority instructions
#[repr(u8)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
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
    /// Authority to set the transfer fee
    TransferFeeConfig,
    /// Authority to withdraw withheld tokens from a mint
    WithheldWithdraw,
    /// Authority to close a mint account
    CloseMint,
    /// Authority to set the interest rate
    InterestRate,
    /// Authority to transfer or burn any tokens for a mint
    PermanentDelegate,
    /// Authority to update confidential transfer mint and aprove accounts for
    /// confidential transfers
    ConfidentialTransferMint,
    /// Authority to set the transfer hook program id
    TransferHookProgramId,
    /// Authority to set the withdraw withheld authority encryption key
    ConfidentialTransferFeeConfig,
    /// Authority to set the metadata address
    MetadataPointer,
    /// Authority to set the group address
    GroupPointer,
    /// Authority to set the group member address
    GroupMemberPointer,
}

impl AuthorityType {
    fn into(&self) -> u8 {
        match self {
            AuthorityType::MintTokens => 0,
            AuthorityType::FreezeAccount => 1,
            AuthorityType::AccountOwner => 2,
            AuthorityType::CloseAccount => 3,
            AuthorityType::TransferFeeConfig => 4,
            AuthorityType::WithheldWithdraw => 5,
            AuthorityType::CloseMint => 6,
            AuthorityType::InterestRate => 7,
            AuthorityType::PermanentDelegate => 8,
            AuthorityType::ConfidentialTransferMint => 9,
            AuthorityType::TransferHookProgramId => 10,
            AuthorityType::ConfidentialTransferFeeConfig => 11,
            AuthorityType::MetadataPointer => 12,
            AuthorityType::GroupPointer => 13,
            AuthorityType::GroupMemberPointer => 14,
        }
    }

    fn from(index: u8) -> Result<Self, ProgramError> {
        match index {
            0 => Ok(AuthorityType::MintTokens),
            1 => Ok(AuthorityType::FreezeAccount),
            2 => Ok(AuthorityType::AccountOwner),
            3 => Ok(AuthorityType::CloseAccount),
            4 => Ok(AuthorityType::TransferFeeConfig),
            5 => Ok(AuthorityType::WithheldWithdraw),
            6 => Ok(AuthorityType::CloseMint),
            7 => Ok(AuthorityType::InterestRate),
            8 => Ok(AuthorityType::PermanentDelegate),
            9 => Ok(AuthorityType::ConfidentialTransferMint),
            10 => Ok(AuthorityType::TransferHookProgramId),
            11 => Ok(AuthorityType::ConfidentialTransferFeeConfig),
            12 => Ok(AuthorityType::MetadataPointer),
            13 => Ok(AuthorityType::GroupPointer),
            14 => Ok(AuthorityType::GroupMemberPointer),
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
#[deprecated(
    since = "4.0.0",
    note = "please use `transfer_checked` or `transfer_checked_with_fee` instead"
)]
pub fn transfer(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_spl_token_program_account(token_program_id)?;
    #[allow(deprecated)]
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;
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
    check_spl_token_program_account(token_program_id)?;

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
    extension_types: &[ExtensionType],
) -> Result<Instruction, ProgramError> {
    check_spl_token_program_account(token_program_id)?;
    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new_readonly(*mint_pubkey, false)],
        data: TokenInstruction::GetAccountDataSize {
            extension_types: extension_types.to_vec(),
        }
        .pack(),
    })
}

/// Creates an `InitializeMintCloseAuthority` instruction
pub fn initialize_mint_close_authority(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    close_authority: Option<&Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let close_authority = close_authority.cloned().into();
    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new(*mint_pubkey, false)],
        data: TokenInstruction::InitializeMintCloseAuthority { close_authority }.pack(),
    })
}

/// Create an `InitializeImmutableOwner` instruction
pub fn initialize_immutable_owner(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_spl_token_program_account(token_program_id)?;
    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new(*token_account, false)],
        data: TokenInstruction::InitializeImmutableOwner.pack(),
    })
}

/// Creates an `AmountToUiAmount` instruction
pub fn amount_to_ui_amount(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    check_spl_token_program_account(token_program_id)?;

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
    check_spl_token_program_account(token_program_id)?;

    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new_readonly(*mint_pubkey, false)],
        data: TokenInstruction::UiAmountToAmount { ui_amount }.pack(),
    })
}

/// Creates a `Reallocate` instruction
pub fn reallocate(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    payer: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    extension_types: &[ExtensionType],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    let mut accounts = Vec::with_capacity(4 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*payer, true));
    accounts.push(AccountMeta::new_readonly(system_program::id(), false));
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
        data: TokenInstruction::Reallocate {
            extension_types: extension_types.to_vec(),
        }
        .pack(),
    })
}

/// Creates a `CreateNativeMint` instruction
pub fn create_native_mint(
    token_program_id: &Pubkey,
    payer: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(crate::native_mint::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: TokenInstruction::CreateNativeMint.pack(),
    })
}

/// Creates an `InitializeNonTransferableMint` instruction
pub fn initialize_non_transferable_mint(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new(*mint_pubkey, false)],
        data: TokenInstruction::InitializeNonTransferableMint.pack(),
    })
}

/// Creates an `InitializePermanentDelegate` instruction
pub fn initialize_permanent_delegate(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    delegate: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    Ok(Instruction {
        program_id: *token_program_id,
        accounts: vec![AccountMeta::new(*mint_pubkey, false)],
        data: TokenInstruction::InitializePermanentDelegate {
            delegate: *delegate,
        }
        .pack(),
    })
}

/// Utility function that checks index is between MIN_SIGNERS and MAX_SIGNERS
pub fn is_valid_signer_index(index: usize) -> bool {
    (MIN_SIGNERS..=MAX_SIGNERS).contains(&index)
}

/// Utility function for decoding just the instruction type
pub fn decode_instruction_type<T: TryFrom<u8>>(input: &[u8]) -> Result<T, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        T::try_from(input[0]).map_err(|_| TokenError::InvalidInstruction.into())
    }
}

/// Utility function for decoding instruction data
///
/// Note: This function expects the entire instruction input, including the
/// instruction type as the first byte.  This makes the code concise and safe
/// at the expense of clarity, allowing flows such as:
///
/// match decode_instruction_type(input)? {
///     InstructionType::First => {
///         let FirstData { ... } = decode_instruction_data(input)?;
///     }
/// }
pub fn decode_instruction_data<T: Pod>(input_with_type: &[u8]) -> Result<&T, ProgramError> {
    if input_with_type.len() != pod_get_packed_len::<T>().saturating_add(1) {
        Err(ProgramError::InvalidInstructionData)
    } else {
        pod_from_bytes(&input_with_type[1..])
    }
}

/// Utility function for encoding instruction data
pub(crate) fn encode_instruction<T: Into<u8>, D: Pod>(
    token_program_id: &Pubkey,
    accounts: Vec<AccountMeta>,
    token_instruction_type: TokenInstruction,
    instruction_type: T,
    instruction_data: &D,
) -> Instruction {
    let mut data = token_instruction_type.pack();
    data.push(T::into(instruction_type));
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    }
}

/// Creates a `WithdrawExcessLamports` Instruction
pub fn withdraw_excess_lamports(
    token_program_id: &Pubkey,
    source_account: &Pubkey,
    destination_account: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    let mut accounts = vec![
        AccountMeta::new(*source_account, false),
        AccountMeta::new(*destination_account, false),
        AccountMeta::new_readonly(*authority, signers.is_empty()),
    ];

    for signer in signers {
        accounts.push(AccountMeta::new_readonly(**signer, true))
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data: TokenInstruction::WithdrawExcessLamports.pack(),
    })
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

        #[allow(deprecated)]
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

        let check = TokenInstruction::GetAccountDataSize {
            extension_types: vec![],
        };
        let packed = check.pack();
        let expect = [21u8];
        assert_eq!(packed, &[21u8]);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::GetAccountDataSize {
            extension_types: vec![
                ExtensionType::TransferFeeConfig,
                ExtensionType::TransferFeeAmount,
            ],
        };
        let packed = check.pack();
        let expect = [21u8, 1, 0, 2, 0];
        assert_eq!(packed, &[21u8, 1, 0, 2, 0]);
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

        let check = TokenInstruction::InitializeMintCloseAuthority {
            close_authority: COption::Some(Pubkey::new_from_array([10u8; 32])),
        };
        let packed = check.pack();
        let mut expect = vec![25u8, 1];
        expect.extend_from_slice(&[10u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::CreateNativeMint;
        let packed = check.pack();
        let expect = vec![31u8];
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializePermanentDelegate {
            delegate: Pubkey::new_from_array([11u8; 32]),
        };
        let packed = check.pack();
        let mut expect = vec![35u8];
        expect.extend_from_slice(&[11u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    macro_rules! test_instruction {
        ($a:ident($($b:tt)*)) => {
            let instruction_v3 = spl_token::instruction::$a($($b)*).unwrap();
            let instruction_2022 = $a($($b)*).unwrap();
            assert_eq!(instruction_v3, instruction_2022);
        }
    }

    #[test]
    fn test_v3_compatibility() {
        let token_program_id = spl_token::id();
        let mint_pubkey = Pubkey::new_unique();
        let mint_authority_pubkey = Pubkey::new_unique();
        let freeze_authority_pubkey = Pubkey::new_unique();
        let decimals = 9u8;

        let account_pubkey = Pubkey::new_unique();
        let owner_pubkey = Pubkey::new_unique();

        let multisig_pubkey = Pubkey::new_unique();
        let signer_pubkeys_vec = vec![Pubkey::new_unique(); MAX_SIGNERS];
        let signer_pubkeys = signer_pubkeys_vec.iter().collect::<Vec<_>>();
        let m = 10u8;

        let source_pubkey = Pubkey::new_unique();
        let destination_pubkey = Pubkey::new_unique();
        let authority_pubkey = Pubkey::new_unique();
        let amount = 1_000_000_000_000;

        let delegate_pubkey = Pubkey::new_unique();
        let owned_pubkey = Pubkey::new_unique();
        let new_authority_pubkey = Pubkey::new_unique();

        let ui_amount = "100000.00";

        test_instruction!(initialize_mint(
            &token_program_id,
            &mint_pubkey,
            &mint_authority_pubkey,
            None,
            decimals,
        ));
        test_instruction!(initialize_mint2(
            &token_program_id,
            &mint_pubkey,
            &mint_authority_pubkey,
            Some(&freeze_authority_pubkey),
            decimals,
        ));

        test_instruction!(initialize_account(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &owner_pubkey,
        ));
        test_instruction!(initialize_account2(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &owner_pubkey,
        ));
        test_instruction!(initialize_account3(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &owner_pubkey,
        ));
        test_instruction!(initialize_multisig(
            &token_program_id,
            &multisig_pubkey,
            &signer_pubkeys,
            m,
        ));
        test_instruction!(initialize_multisig2(
            &token_program_id,
            &multisig_pubkey,
            &signer_pubkeys,
            m,
        ));
        #[allow(deprecated)]
        {
            test_instruction!(transfer(
                &token_program_id,
                &source_pubkey,
                &destination_pubkey,
                &authority_pubkey,
                &signer_pubkeys,
                amount
            ));
        }
        test_instruction!(transfer_checked(
            &token_program_id,
            &source_pubkey,
            &mint_pubkey,
            &destination_pubkey,
            &authority_pubkey,
            &signer_pubkeys,
            amount,
            decimals,
        ));
        test_instruction!(approve(
            &token_program_id,
            &source_pubkey,
            &delegate_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
            amount
        ));
        test_instruction!(approve_checked(
            &token_program_id,
            &source_pubkey,
            &mint_pubkey,
            &delegate_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
            amount,
            decimals
        ));
        test_instruction!(revoke(
            &token_program_id,
            &source_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
        ));

        // set_authority
        {
            let instruction_v3 = spl_token::instruction::set_authority(
                &token_program_id,
                &owned_pubkey,
                Some(&new_authority_pubkey),
                spl_token::instruction::AuthorityType::AccountOwner,
                &owner_pubkey,
                &signer_pubkeys,
            )
            .unwrap();
            let instruction_2022 = set_authority(
                &token_program_id,
                &owned_pubkey,
                Some(&new_authority_pubkey),
                AuthorityType::AccountOwner,
                &owner_pubkey,
                &signer_pubkeys,
            )
            .unwrap();
            assert_eq!(instruction_v3, instruction_2022);
        }

        test_instruction!(mint_to(
            &token_program_id,
            &mint_pubkey,
            &account_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
            amount,
        ));
        test_instruction!(mint_to_checked(
            &token_program_id,
            &mint_pubkey,
            &account_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
            amount,
            decimals,
        ));
        test_instruction!(burn(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &authority_pubkey,
            &signer_pubkeys,
            amount,
        ));
        test_instruction!(burn_checked(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &authority_pubkey,
            &signer_pubkeys,
            amount,
            decimals,
        ));
        test_instruction!(close_account(
            &token_program_id,
            &account_pubkey,
            &destination_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
        ));
        test_instruction!(freeze_account(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
        ));
        test_instruction!(thaw_account(
            &token_program_id,
            &account_pubkey,
            &mint_pubkey,
            &owner_pubkey,
            &signer_pubkeys,
        ));
        test_instruction!(sync_native(&token_program_id, &account_pubkey,));

        // get_account_data_size
        {
            let instruction_v3 =
                spl_token::instruction::get_account_data_size(&token_program_id, &mint_pubkey)
                    .unwrap();
            let instruction_2022 =
                get_account_data_size(&token_program_id, &mint_pubkey, &[]).unwrap();
            assert_eq!(instruction_v3, instruction_2022);
        }

        test_instruction!(initialize_immutable_owner(
            &token_program_id,
            &account_pubkey,
        ));

        test_instruction!(amount_to_ui_amount(&token_program_id, &mint_pubkey, amount,));

        test_instruction!(ui_amount_to_amount(
            &token_program_id,
            &mint_pubkey,
            ui_amount,
        ));
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
