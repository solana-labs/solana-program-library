//! Struct for managing extra required account configs, ie. defining accounts
//! required for your interface program, which can be  `AccountMeta`s - which
//! have fixed addresses - or PDAs - which have addresses derived from a
//! collection of seeds

use {
    crate::{error::AccountResolutionError, pubkey_data::PubkeyData, seeds::Seed},
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo,
        instruction::AccountMeta,
        program_error::ProgramError,
        pubkey::{Pubkey, PUBKEY_BYTES},
    },
    spl_pod::primitives::PodBool,
};

/// Resolve a program-derived address (PDA) from the instruction data
/// and the accounts that have already been resolved
fn resolve_pda<'a, F>(
    seeds: &[Seed],
    instruction_data: &[u8],
    program_id: &Pubkey,
    get_account_key_data_fn: F,
) -> Result<Pubkey, ProgramError>
where
    F: Fn(usize) -> Option<(&'a Pubkey, Option<&'a [u8]>)>,
{
    let mut pda_seeds: Vec<&[u8]> = vec![];
    for config in seeds {
        match config {
            Seed::Uninitialized => (),
            Seed::Literal { bytes } => pda_seeds.push(bytes),
            Seed::InstructionData { index, length } => {
                let arg_start = *index as usize;
                let arg_end = arg_start + *length as usize;
                if arg_end > instruction_data.len() {
                    return Err(AccountResolutionError::InstructionDataTooSmall.into());
                }
                pda_seeds.push(&instruction_data[arg_start..arg_end]);
            }
            Seed::AccountKey { index } => {
                let account_index = *index as usize;
                let address = get_account_key_data_fn(account_index)
                    .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?
                    .0;
                pda_seeds.push(address.as_ref());
            }
            Seed::AccountData {
                account_index,
                data_index,
                length,
            } => {
                let account_index = *account_index as usize;
                let account_data = get_account_key_data_fn(account_index)
                    .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?
                    .1
                    .ok_or::<ProgramError>(AccountResolutionError::AccountDataNotFound.into())?;
                let arg_start = *data_index as usize;
                let arg_end = arg_start + *length as usize;
                if account_data.len() < arg_end {
                    return Err(AccountResolutionError::AccountDataTooSmall.into());
                }
                pda_seeds.push(&account_data[arg_start..arg_end]);
            }
        }
    }
    Ok(Pubkey::find_program_address(&pda_seeds, program_id).0)
}

/// Resolve a pubkey from a pubkey data configuration.
fn resolve_key_data<'a, F>(
    key_data: &PubkeyData,
    instruction_data: &[u8],
    get_account_key_data_fn: F,
) -> Result<Pubkey, ProgramError>
where
    F: Fn(usize) -> Option<(&'a Pubkey, Option<&'a [u8]>)>,
{
    match key_data {
        PubkeyData::Uninitialized => Err(ProgramError::InvalidAccountData),
        PubkeyData::InstructionData { index } => {
            let key_start = *index as usize;
            let key_end = key_start + PUBKEY_BYTES;
            if key_end > instruction_data.len() {
                return Err(AccountResolutionError::InstructionDataTooSmall.into());
            }
            Ok(Pubkey::new_from_array(
                instruction_data[key_start..key_end].try_into().unwrap(),
            ))
        }
        PubkeyData::AccountData {
            account_index,
            data_index,
        } => {
            let account_index = *account_index as usize;
            let account_data = get_account_key_data_fn(account_index)
                .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?
                .1
                .ok_or::<ProgramError>(AccountResolutionError::AccountDataNotFound.into())?;
            let arg_start = *data_index as usize;
            let arg_end = arg_start + PUBKEY_BYTES;
            if account_data.len() < arg_end {
                return Err(AccountResolutionError::AccountDataTooSmall.into());
            }
            Ok(Pubkey::new_from_array(
                account_data[arg_start..arg_end].try_into().unwrap(),
            ))
        }
    }
}

/// `Pod` type for defining a required account in a validation account.
///
/// This can be any of the following:
///
/// * A standard `AccountMeta`
/// * A PDA (with seed configurations)
/// * A pubkey stored in some data (account or instruction data)
///
/// Can be used in TLV-encoded data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ExtraAccountMeta {
    /// Discriminator to tell whether this represents a standard
    /// `AccountMeta`, PDA, or pubkey data.
    pub discriminator: u8,
    /// This `address_config` field can either be the pubkey of the account,
    /// the seeds used to derive the pubkey from provided inputs (PDA), or the
    /// data used to derive the pubkey (account or instruction data).
    pub address_config: [u8; 32],
    /// Whether the account should sign
    pub is_signer: PodBool,
    /// Whether the account should be writable
    pub is_writable: PodBool,
}
/// Helper used to know when the top bit is set, to interpret the
/// discriminator as an index rather than as a type
const U8_TOP_BIT: u8 = 1 << 7;
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

    /// Create a `ExtraAccountMeta` from a pubkey data configuration.
    pub fn new_with_pubkey_data(
        key_data: &PubkeyData,
        is_signer: bool,
        is_writable: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: 2,
            address_config: PubkeyData::pack_into_address_config(key_data)?,
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        })
    }

    /// Create a `ExtraAccountMeta` from a list of seed configurations,
    /// representing a PDA for an external program
    ///
    /// This PDA belongs to a program elsewhere in the account list, rather
    /// than the executing program. For a PDA on the executing program, use
    /// `ExtraAccountMeta::new_with_seeds`.
    pub fn new_external_pda_with_seeds(
        program_index: u8,
        seeds: &[Seed],
        is_signer: bool,
        is_writable: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            discriminator: program_index
                .checked_add(U8_TOP_BIT)
                .ok_or(AccountResolutionError::InvalidSeedConfig)?,
            address_config: Seed::pack_into_address_config(seeds)?,
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        })
    }

    /// Resolve an `ExtraAccountMeta` into an `AccountMeta`, potentially
    /// resolving a program-derived address (PDA) if necessary
    pub fn resolve<'a, F>(
        &self,
        instruction_data: &[u8],
        program_id: &Pubkey,
        get_account_key_data_fn: F,
    ) -> Result<AccountMeta, ProgramError>
    where
        F: Fn(usize) -> Option<(&'a Pubkey, Option<&'a [u8]>)>,
    {
        match self.discriminator {
            0 => AccountMeta::try_from(self),
            x if x == 1 || x >= U8_TOP_BIT => {
                let program_id = if x == 1 {
                    program_id
                } else {
                    get_account_key_data_fn(x.saturating_sub(U8_TOP_BIT) as usize)
                        .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?
                        .0
                };
                let seeds = Seed::unpack_address_config(&self.address_config)?;
                Ok(AccountMeta {
                    pubkey: resolve_pda(
                        &seeds,
                        instruction_data,
                        program_id,
                        get_account_key_data_fn,
                    )?,
                    is_signer: self.is_signer.into(),
                    is_writable: self.is_writable.into(),
                })
            }
            2 => {
                let key_data = PubkeyData::unpack(&self.address_config)?;
                Ok(AccountMeta {
                    pubkey: resolve_key_data(&key_data, instruction_data, get_account_key_data_fn)?,
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
                pubkey: Pubkey::from(pod.address_config),
                is_signer: pod.is_signer.into(),
                is_writable: pod.is_writable.into(),
            })
        } else {
            Err(AccountResolutionError::AccountTypeNotAccountMeta.into())
        }
    }
}
