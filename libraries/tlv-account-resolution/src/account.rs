//! Struct for managing extra required account configs, ie. defining accounts
//! required for your interface program, which can be  `AccountMeta`s - which
//! have fixed addresses - or PDAs - which have addresses derived from a
//! collection of seeds

use {
    crate::{error::AccountResolutionError, seeds::Seed},
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_type_length_value::pod::PodBool,
    std::future::Future,
};

/// Type representing the output of an account fetching function, for easy
/// chaining between APIs
pub type AccountDataResult = Result<Option<Vec<u8>>, AccountFetchError>;
/// Generic error type that can come out of any client while fetching account data
pub type AccountFetchError = Box<dyn std::error::Error + Send + Sync>;

/// Resolve a program-derived address (PDA) from the instruction data
/// and the accounts that have already been resolved
///
/// This function must be used off-chain.
async fn resolve_pda_offchain<F, Fut>(
    seeds: &[Seed],
    get_account_data_fn: F,
    accounts: &[AccountMeta],
    instruction_data: &[u8],
    program_id: &Pubkey,
) -> Result<Pubkey, ProgramError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountDataResult>,
{
    //
    // TODO: Refactor to minimize copying!
    // The dropped reference problem with the account data is the culprit
    //
    let mut pda_seeds: Vec<Vec<u8>> = vec![];
    for config in seeds {
        match config {
            Seed::Uninitialized => (),
            Seed::Literal { bytes } => pda_seeds.push(bytes.clone()),
            Seed::InstructionData { index, length } => {
                let arg_start = *index as usize;
                let arg_end = arg_start + *length as usize;
                pda_seeds.push(instruction_data[arg_start..arg_end].to_vec());
            }
            Seed::AccountKey { index } => {
                let account_index = *index as usize;
                let account = accounts
                    .get(account_index)
                    .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?;
                pda_seeds.push(account.pubkey.to_bytes().to_vec());
            }
            Seed::AccountData {
                account_index,
                data_index,
                length,
            } => {
                let account_index = *account_index as usize;
                let account = accounts
                    .get(account_index)
                    .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?;
                let account_data = get_account_data_fn(account.pubkey)
                    .await
                    .map_err(|_| ProgramError::from(AccountResolutionError::AccountFetchFailed))?
                    .ok_or::<ProgramError>(AccountResolutionError::AccountFetchFailed.into())?;
                let arg_start = *data_index as usize;
                let arg_end = arg_start + *length as usize;
                if account_data.len() < arg_end {
                    return Err(AccountResolutionError::AccountDataTooSmall.into());
                }
                pda_seeds.push(account_data[arg_start..arg_end].to_vec());
            }
        }
    }
    Ok(Pubkey::find_program_address(
        pda_seeds
            .iter()
            .map(|v| v.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice(),
        program_id,
    )
    .0)
}

/// Resolve a program-derived address (PDA) from the instruction data
/// and the accounts that have already been resolved
///
/// This function should be used on-chain, but can also be used off-chain.
fn resolve_pda_onchain(
    seeds: &[Seed],
    accounts: &[AccountInfo],
    instruction_data: &[u8],
    program_id: &Pubkey,
) -> Result<Pubkey, ProgramError> {
    //
    // TODO: Refactor to minimize copying!
    // The dropped reference problem with the account data is the culprit
    //
    let mut pda_seeds: Vec<Vec<u8>> = vec![];
    for config in seeds {
        match config {
            Seed::Uninitialized => (),
            Seed::Literal { bytes } => pda_seeds.push(bytes.clone()),
            Seed::InstructionData { index, length } => {
                let arg_start = *index as usize;
                let arg_end = arg_start + *length as usize;
                pda_seeds.push(instruction_data[arg_start..arg_end].to_vec());
            }
            Seed::AccountKey { index } => {
                let account_index = *index as usize;
                let account = accounts
                    .get(account_index)
                    .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?;
                pda_seeds.push(account.key.to_bytes().to_vec());
            }
            Seed::AccountData {
                account_index,
                data_index,
                length,
            } => {
                let account_index = *account_index as usize;
                let account = accounts
                    .get(account_index)
                    .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?;
                let account_data = account.try_borrow_data()?;
                let arg_start = *data_index as usize;
                let arg_end = arg_start + *length as usize;
                if account_data.len() < arg_end {
                    return Err(AccountResolutionError::AccountDataTooSmall.into());
                }
                pda_seeds.push(account_data[arg_start..arg_end].to_vec());
            }
        }
    }
    Ok(Pubkey::find_program_address(
        pda_seeds
            .iter()
            .map(|v| v.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice(),
        program_id,
    )
    .0)
}

/// `Pod` type for defining a required account in a validation account.
///
/// This can either be a standard `AccountMeta` or a PDA.
/// Can be used in TLV-encoded data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ExtraAccountMeta {
    /// Discriminator to tell whether this represents a standard
    /// `AccountMeta` or a PDA
    pub discriminator: u8,
    /// This `address_config` field can either be the pubkey of the account
    /// or the seeds used to derive the pubkey from provided inputs
    pub address_config: [u8; 32],
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}
impl ExtraAccountMeta {
    /// Create a `ExtraAccountMeta` from a public key,
    /// thus representing a standard `AccountMeta`
    pub fn new_with_pubkey(
        pubkey: &Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: 0,
            address_config: pubkey.to_bytes(),
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        })
    }

    /// Create a `ExtraAccountMeta` from a list of seed configurations,
    /// thus representing a PDA
    pub fn new_with_seeds(
        seeds: &[Seed],
        is_signer: bool,
        is_writable: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: 1,
            address_config: Seed::pack_into_address_config(seeds)?,
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        })
    }

    /// **Off-Chain**: Must be used off-chain.
    ///
    /// Resolve an `ExtraAccountMeta` into an `AccountMeta`, potentially
    /// resolving a program-derived address (PDA) if necessary.
    ///
    /// This function must be used off-chain.
    pub async fn resolve_offchain<F, Fut>(
        &self,
        get_account_data_fn: F,
        instruction_accounts: &[AccountMeta],
        instruction_data: &[u8],
        program_id: &Pubkey,
    ) -> Result<AccountMeta, ProgramError>
    where
        F: Fn(Pubkey) -> Fut,
        Fut: Future<Output = AccountDataResult>,
    {
        match self.discriminator {
            0 => AccountMeta::try_from(self),
            1 => {
                let seeds = Seed::unpack_address_config(&self.address_config)?;
                Ok(AccountMeta {
                    pubkey: resolve_pda_offchain(
                        &seeds,
                        get_account_data_fn,
                        instruction_accounts,
                        instruction_data,
                        program_id,
                    )
                    .await?,
                    is_signer: self.is_signer.into(),
                    is_writable: self.is_writable.into(),
                })
            }
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    /// Resolve an `ExtraAccountMeta` into an `AccountMeta`, potentially
    /// resolving a program-derived address (PDA) if necessary.
    ///
    /// This function should be used on-chain, but can also be used off-chain.
    pub fn resolve_onchain(
        &self,
        instruction_accounts: &[AccountInfo],
        instruction_data: &[u8],
        program_id: &Pubkey,
    ) -> Result<AccountMeta, ProgramError> {
        match self.discriminator {
            0 => AccountMeta::try_from(self),
            1 => {
                let seeds = Seed::unpack_address_config(&self.address_config)?;
                Ok(AccountMeta {
                    pubkey: resolve_pda_onchain(
                        &seeds,
                        instruction_accounts,
                        instruction_data,
                        program_id,
                    )?,
                    is_signer: self.is_signer.into(),
                    is_writable: self.is_writable.into(),
                })
            }
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

impl From<&AccountMeta> for ExtraAccountMeta {
    fn from(meta: &AccountMeta) -> Self {
        Self {
            discriminator: 0,
            address_config: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer.into(),
            is_writable: meta.is_writable.into(),
        }
    }
}
impl From<AccountMeta> for ExtraAccountMeta {
    fn from(meta: AccountMeta) -> Self {
        ExtraAccountMeta::from(&meta)
    }
}
impl From<&AccountInfo<'_>> for ExtraAccountMeta {
    fn from(account_info: &AccountInfo) -> Self {
        Self {
            discriminator: 0,
            address_config: account_info.key.to_bytes(),
            is_signer: account_info.is_signer.into(),
            is_writable: account_info.is_writable.into(),
        }
    }
}
impl From<AccountInfo<'_>> for ExtraAccountMeta {
    fn from(account_info: AccountInfo) -> Self {
        ExtraAccountMeta::from(&account_info)
    }
}

impl TryFrom<&ExtraAccountMeta> for AccountMeta {
    type Error = ProgramError;

    fn try_from(pod: &ExtraAccountMeta) -> Result<Self, Self::Error> {
        if pod.discriminator == 0 {
            Ok(AccountMeta {
                pubkey: Pubkey::try_from(pod.address_config)
                    .map_err(|_| ProgramError::from(AccountResolutionError::InvalidPubkey))?,
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::AccountTypeNotAccountMeta.into())
        }
    }
}
