//! Multisignatures handling routines

use {
    crate::{
        error::FarmError,
        id::{main_router_admin, zero},
        pack::*,
        program::account,
        traits::*,
    },
    ahash::AHasher,
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::hash::Hasher,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Multisig {
    pub num_signers: u8,
    pub num_signed: u8,
    pub min_signatures: u8,
    pub instruction_accounts_len: u8,
    pub instruction_data_len: u16,
    pub instruction_hash: u64,
    #[serde(serialize_with = "pubkey_slice_serialize")]
    pub signers: [Pubkey; Multisig::MAX_SIGNERS],
    pub signed: [bool; Multisig::MAX_SIGNERS],
}

/// Returns instruction accounts and data hash.
/// Hash is not cryptographic and is meant to perform a fast check that admins are signing
/// the same instruction.
pub fn get_instruction_hash(instruction_accounts: &[AccountInfo], instruction_data: &[u8]) -> u64 {
    let mut hasher = AHasher::new_with_keys(697533735114380, 537268678243635);
    for account in instruction_accounts {
        hasher.write(account.key.as_ref());
    }
    if !instruction_data.is_empty() {
        hasher.write(instruction_data);
    }
    hasher.finish()
}

/// Initializes multisig PDA with a new set of signers
pub fn set_signers(
    multisig_account: &AccountInfo,
    admin_signers: &[AccountInfo],
    min_signatures: u8,
) -> Result<usize, ProgramError> {
    if admin_signers.is_empty() || min_signatures == 0 {
        msg!("Error: At least one signer is required");
        return Err(ProgramError::MissingRequiredSignature);
    }
    if (min_signatures as usize) > admin_signers.len() {
        msg!(
            "Error: Number of min signatures ({}) exceeded number of signers ({})",
            min_signatures,
            admin_signers.len(),
        );
        return Err(ProgramError::InvalidArgument);
    }
    if admin_signers.len() > Multisig::MAX_SIGNERS {
        msg!(
            "Error: Number of signers ({}) exceeded max ({})",
            admin_signers.len(),
            Multisig::MAX_SIGNERS
        );
        return Err(ProgramError::InvalidArgument);
    }

    let mut signers: [Pubkey; Multisig::MAX_SIGNERS] = Default::default();
    let mut signed: [bool; Multisig::MAX_SIGNERS] = Default::default();

    for idx in 0..admin_signers.len() as usize {
        if signers.contains(admin_signers[idx].key) {
            msg!("Error: Duplicate signer {}", admin_signers[idx].key);
            return Err(FarmError::IncorrectAccountAddress.into());
        }
        signers[idx] = *admin_signers[idx].key;
        signed[idx] = false;
    }

    Multisig {
        num_signers: admin_signers.len() as u8,
        num_signed: 0,
        min_signatures,
        instruction_accounts_len: 0,
        instruction_data_len: 0,
        instruction_hash: 0,
        signers,
        signed,
    }
    .pack(*multisig_account.try_borrow_mut_data()?)
}

/// Signs multisig and returns Ok(0) if there are enough signatures to continue or Ok(signatures_left) otherwise.
/// If Err() is returned then signature was not recognized and transaction must be aborted.
pub fn sign_multisig(
    multisig_account: &AccountInfo,
    signer_account: &AccountInfo,
    fallback_admin_account: &Pubkey,
    instruction_accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<u8, ProgramError> {
    // return early if not a signer
    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // if multisig is empty check that signer is a main router admin
    if !account::exists(multisig_account)? {
        if signer_account.key != fallback_admin_account {
            msg!("Error: Account is not authorized to sign this instruction");
            return Err(FarmError::AccountNotAuthorized.into());
        } else {
            return Ok(0);
        }
    }

    // unpack multisig PDA
    let mut multisig = account::unpack::<Multisig>(multisig_account, "multisig_account")?;

    // find index of current signer or return error if not found
    let signer_idx = if let Ok(idx) = get_signer_index(&multisig, signer_account.key) {
        idx
    } else {
        msg!("Error: Account is not a signer in this multisig");
        return Err(FarmError::AccountNotAuthorized.into());
    };

    // if single signer return Ok to continue
    if multisig.num_signers <= 1 {
        return Ok(0);
    }

    let instruction_hash = get_instruction_hash(instruction_accounts, instruction_data);
    if instruction_hash != multisig.instruction_hash
        || instruction_accounts.len() != multisig.instruction_accounts_len as usize
        || instruction_data.len() != multisig.instruction_data_len as usize
    {
        // if this is a new instruction reset the data
        multisig.num_signed = 1;
        multisig.instruction_accounts_len = instruction_accounts.len() as u8;
        multisig.instruction_data_len = instruction_data.len() as u16;
        multisig.instruction_hash = instruction_hash;
        multisig.signed.fill(false);
        multisig.signed[signer_idx] = true;
        multisig.pack(*multisig_account.try_borrow_mut_data()?)?;

        Ok(multisig.min_signatures - 1)
    } else if multisig.signed[signer_idx] {
        msg!("Error: Account has already signed this instruction");
        Err(FarmError::AlreadySigned.into())
    } else if multisig.num_signed < multisig.min_signatures {
        // count the signature in
        multisig.num_signed += 1;
        multisig.signed[signer_idx] = true;
        multisig.pack(*multisig_account.try_borrow_mut_data()?)?;

        if multisig.num_signed == multisig.min_signatures {
            Ok(0)
        } else {
            Ok(multisig.min_signatures - multisig.num_signed)
        }
    } else {
        msg!("Error: This instruction has already been executed");
        Err(FarmError::AlreadyExecuted.into())
    }
}

/// Removes admin signature from the multisig
pub fn unsign_multisig(
    multisig_account: &AccountInfo,
    signer_account: &AccountInfo,
) -> ProgramResult {
    // return early if not a signer
    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // if multisig doesn't exist return
    if !account::exists(multisig_account)? {
        return Ok(());
    }

    // unpack multisig PDA
    let mut multisig = account::unpack::<Multisig>(multisig_account, "multisig_account")?;

    // if single signer return
    if multisig.num_signers <= 1 || multisig.num_signed == 0 {
        return Ok(());
    }

    // find index of current signer or return error if not found
    let signer_idx = if let Ok(idx) = get_signer_index(&multisig, signer_account.key) {
        idx
    } else {
        msg!("Error: Account is not a signer in this multisig");
        return Err(FarmError::AccountNotAuthorized.into());
    };

    // if not signed by this account return
    if !multisig.signed[signer_idx] {
        return Ok(());
    }

    // remove signature
    multisig.num_signed -= 1;
    multisig.signed[signer_idx] = false;

    multisig.pack(*multisig_account.try_borrow_mut_data()?)?;

    Ok(())
}

/// Returns the array index of the provided signer
pub fn get_signer_index(multisig: &Multisig, signer: &Pubkey) -> Result<usize, ProgramError> {
    for i in 0..Multisig::MAX_SIGNERS {
        if &multisig.signers[i] == signer {
            return Ok(i);
        }
    }
    Err(FarmError::AccountNotAuthorized.into())
}

/// Checks if provided account is one of multisig signers
pub fn is_signer(
    multisig_account: &AccountInfo,
    fallback_admin_account: &Pubkey,
    key: &Pubkey,
) -> Result<bool, ProgramError> {
    if !account::exists(multisig_account)? {
        return Ok(key == fallback_admin_account);
    }

    // unpack multisig PDA
    let multisig = account::unpack::<Multisig>(multisig_account, "multisig_account")?;

    Ok(get_signer_index(&multisig, key).is_ok())
}

impl Multisig {
    pub const MAX_SIGNERS: usize = 6;
    pub const LEN: usize = 14 + Multisig::MAX_SIGNERS * 33;
}

impl Packed for Multisig {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, Multisig::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            num_signers_out,
            num_signed_out,
            min_signatures_out,
            instruction_accounts_len_out,
            instruction_data_len_out,
            instruction_hash_out,
            signers_out,
            signed_out,
        ) = mut_array_refs![
            output,
            1,
            1,
            1,
            1,
            2,
            8,
            32usize * Multisig::MAX_SIGNERS,
            Multisig::MAX_SIGNERS
        ];

        num_signers_out[0] = self.num_signers;
        num_signed_out[0] = self.num_signed;
        min_signatures_out[0] = self.min_signatures;
        instruction_accounts_len_out[0] = self.instruction_accounts_len;
        *instruction_data_len_out = self.instruction_data_len.to_le_bytes();
        *instruction_hash_out = self.instruction_hash.to_le_bytes();
        for idx in 0..self.num_signers as usize {
            signers_out[idx * 32..idx * 32 + 32].copy_from_slice(self.signers[idx].as_ref());
            signed_out[idx] = self.signed[idx] as u8;
        }
        signers_out[(self.num_signers * 32) as usize..].fill(0);
        signed_out[self.num_signers as usize..].fill(0);

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, Multisig::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            num_signers,
            num_signed,
            min_signatures,
            instruction_accounts_len,
            instruction_data_len,
            instruction_hash,
            signers_ref,
            signed_ref,
        ) = array_refs![
            input,
            1,
            1,
            1,
            1,
            2,
            8,
            32 * Multisig::MAX_SIGNERS,
            Multisig::MAX_SIGNERS
        ];

        let mut signers: [Pubkey; Multisig::MAX_SIGNERS] = Default::default();
        let mut signed: [bool; Multisig::MAX_SIGNERS] = Default::default();
        let num_signers = num_signers[0];
        let num_signed = num_signed[0];
        let min_signatures = min_signatures[0];
        if num_signers as usize > Multisig::MAX_SIGNERS
            || num_signed > num_signers
            || min_signatures > num_signers
            || (num_signers > 0 && min_signatures == 0)
        {
            return Err(ProgramError::InvalidAccountData);
        }

        for idx in 0..num_signers as usize {
            signers[idx] = Pubkey::new(&signers_ref[idx * 32..idx * 32 + 32]);
            signed[idx] = match signed_ref[idx] {
                0 => false,
                1 => true,
                _ => return Err(ProgramError::InvalidAccountData),
            };
        }
        for idx in (num_signers as usize)..Multisig::MAX_SIGNERS {
            signers[idx] = zero::id();
            signed[idx] = false;
        }

        Ok(Self {
            num_signers,
            num_signed,
            min_signatures,
            instruction_accounts_len: instruction_accounts_len[0],
            instruction_data_len: u16::from_le_bytes(*instruction_data_len),
            instruction_hash: u64::from_le_bytes(*instruction_hash),
            signers,
            signed,
        })
    }
}

impl Default for Multisig {
    fn default() -> Self {
        Self {
            num_signers: 1,
            num_signed: 0,
            min_signatures: 1,
            instruction_accounts_len: 0,
            instruction_data_len: 0,
            instruction_hash: 0,
            signers: [
                main_router_admin::id(),
                zero::id(),
                zero::id(),
                zero::id(),
                zero::id(),
                zero::id(),
            ],
            signed: [false, false, false, false, false, false],
        }
    }
}

impl std::fmt::Display for Multisig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_serialization() {
        let ri1 = Multisig {
            num_signers: 2,
            num_signed: 1,
            min_signatures: 2,
            instruction_accounts_len: 3,
            instruction_data_len: 123,
            instruction_hash: 123456789,
            signers: [
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                zero::id(),
                zero::id(),
                zero::id(),
                zero::id(),
            ],
            signed: [false, true, false, false, false, false],
        };

        let vec = ri1.to_vec().unwrap();

        let ri2 = Multisig::unpack(&vec[..]).unwrap();

        assert_eq!(ri1, ri2);

        let ri1 = Multisig {
            num_signers: 6,
            num_signed: 1,
            min_signatures: 6,
            instruction_accounts_len: 10,
            instruction_data_len: 123,
            instruction_hash: 987654321,
            signers: [
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
            ],
            signed: [false, false, false, false, false, true],
        };

        let vec = ri1.to_vec().unwrap();

        let ri2 = Multisig::unpack(&vec[..]).unwrap();

        assert_eq!(ri1, ri2);

        let ri1 = Multisig {
            num_signers: 0,
            num_signed: 0,
            min_signatures: 0,
            instruction_accounts_len: 0,
            instruction_data_len: 0,
            instruction_hash: 0,
            signers: [
                zero::id(),
                zero::id(),
                zero::id(),
                zero::id(),
                zero::id(),
                zero::id(),
            ],
            signed: [false, false, false, false, false, false],
        };

        let vec = ri1.to_vec().unwrap();

        let ri2 = Multisig::unpack(&vec[..]).unwrap();

        assert_eq!(ri1, ri2);
    }
}
