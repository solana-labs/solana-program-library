//! Multisig state and routines

use {
    crate::{error::PerpetualsError, math},
    ahash::AHasher,
    anchor_lang::prelude::*,
    std::hash::Hasher,
};

#[repr(packed)]
#[account(zero_copy)]
#[derive(Default)]
pub struct Multisig {
    pub num_signers: u8,
    pub num_signed: u8,
    pub min_signatures: u8,
    pub instruction_accounts_len: u8,
    pub instruction_data_len: u16,
    pub instruction_hash: u64,
    pub signers: [Pubkey; 6], // Multisig::MAX_SIGNERS
    pub signed: [bool; 6],    // Multisig::MAX_SIGNERS
    pub bump: u8,
}

pub enum AdminInstruction {
    AddPool,
    RemovePool,
    AddCustody,
    RemoveCustody,
    SetAdminSigners,
    SetCustodyConfig,
    SetPermissions,
    SetBorrowRate,
    WithdrawFees,
    WithdrawSolFees,
    SetTestOraclePrice,
    SetTestTime,
    UpgradeCustody,
}

impl Multisig {
    pub const MAX_SIGNERS: usize = 6;
    pub const LEN: usize = 8 + std::mem::size_of::<Multisig>();

    /// Returns instruction accounts and data hash.
    /// Hash is not cryptographic and is meant to perform a fast check that admins are signing
    /// the same instruction.
    pub fn get_instruction_hash(
        instruction_accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> u64 {
        let mut hasher = AHasher::new_with_keys(697533735114380, 537268678243635);
        for account in instruction_accounts {
            hasher.write(account.key.as_ref());
        }
        if !instruction_data.is_empty() {
            hasher.write(instruction_data);
        }
        hasher.finish()
    }

    /// Returns all accounts for the given context
    pub fn get_account_infos<'info, T: ToAccountInfos<'info>>(
        ctx: &Context<'_, '_, '_, 'info, T>,
    ) -> Vec<AccountInfo<'info>> {
        let mut infos = ctx.accounts.to_account_infos();
        infos.extend_from_slice(ctx.remaining_accounts);
        infos
    }

    /// Returns serialized instruction data
    pub fn get_instruction_data<T: AnchorSerialize>(
        instruction_type: AdminInstruction,
        params: &T,
    ) -> Result<Vec<u8>> {
        let mut res = vec![];
        AnchorSerialize::serialize(&params, &mut res)?;
        res.push(instruction_type as u8);
        Ok(res)
    }

    /// Initializes multisig PDA with a new set of signers
    pub fn set_signers(&mut self, admin_signers: &[AccountInfo], min_signatures: u8) -> Result<()> {
        if admin_signers.is_empty() || min_signatures == 0 {
            msg!("Error: At least one signer is required");
            return Err(ProgramError::MissingRequiredSignature.into());
        }
        if (min_signatures as usize) > admin_signers.len() {
            msg!(
                "Error: Number of min signatures ({}) exceeded number of signers ({})",
                min_signatures,
                admin_signers.len(),
            );
            return Err(ProgramError::InvalidArgument.into());
        }
        if admin_signers.len() > Multisig::MAX_SIGNERS {
            msg!(
                "Error: Number of signers ({}) exceeded max ({})",
                admin_signers.len(),
                Multisig::MAX_SIGNERS
            );
            return Err(ProgramError::InvalidArgument.into());
        }

        let mut signers: [Pubkey; Multisig::MAX_SIGNERS] = Default::default();
        let mut signed: [bool; Multisig::MAX_SIGNERS] = Default::default();

        for idx in 0..admin_signers.len() {
            if signers.contains(admin_signers[idx].key) {
                msg!("Error: Duplicate signer {}", admin_signers[idx].key);
                return Err(ProgramError::InvalidArgument.into());
            }
            signers[idx] = *admin_signers[idx].key;
            signed[idx] = false;
        }

        *self = Multisig {
            num_signers: admin_signers.len() as u8,
            num_signed: 0,
            min_signatures,
            instruction_accounts_len: 0,
            instruction_data_len: 0,
            instruction_hash: 0,
            signers,
            signed,
            bump: self.bump,
        };

        Ok(())
    }

    /// Signs multisig and returns Ok(0) if there are enough signatures to continue or Ok(signatures_left) otherwise.
    /// If Err() is returned then signature was not recognized and transaction must be aborted.
    pub fn sign_multisig(
        &mut self,
        signer_account: &AccountInfo,
        instruction_accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> Result<u8> {
        // return early if not a signer
        if !signer_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into());
        }

        // find index of current signer or return error if not found
        let signer_idx = if let Ok(idx) = self.get_signer_index(signer_account.key) {
            idx
        } else {
            return err!(PerpetualsError::MultisigAccountNotAuthorized);
        };

        // if single signer return Ok to continue
        if self.num_signers <= 1 {
            return Ok(0);
        }

        let instruction_hash =
            Multisig::get_instruction_hash(instruction_accounts, instruction_data);
        if instruction_hash != self.instruction_hash
            || instruction_accounts.len() != self.instruction_accounts_len as usize
            || instruction_data.len() != self.instruction_data_len as usize
        {
            // if this is a new instruction reset the data
            self.num_signed = 1;
            self.instruction_accounts_len = instruction_accounts.len() as u8;
            self.instruction_data_len = instruction_data.len() as u16;
            self.instruction_hash = instruction_hash;
            self.signed.fill(false);
            self.signed[signer_idx] = true;
            //multisig.pack(*multisig_account.try_borrow_mut_data()?)?;

            math::checked_sub(self.min_signatures, 1)
        } else if self.signed[signer_idx] {
            err!(PerpetualsError::MultisigAlreadySigned)
        } else if self.num_signed < self.min_signatures {
            // count the signature in
            self.num_signed += 1;
            self.signed[signer_idx] = true;

            if self.num_signed == self.min_signatures {
                Ok(0)
            } else {
                math::checked_sub(self.min_signatures, self.num_signed)
            }
        } else {
            err!(PerpetualsError::MultisigAlreadyExecuted)
        }
    }

    /// Removes admin signature from the multisig
    pub fn unsign_multisig(&mut self, signer_account: &AccountInfo) -> Result<()> {
        // return early if not a signer
        if !signer_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into());
        }

        // if single signer return
        if self.num_signers <= 1 || self.num_signed == 0 {
            return Ok(());
        }

        // find index of current signer or return error if not found
        let signer_idx = if let Ok(idx) = self.get_signer_index(signer_account.key) {
            idx
        } else {
            return err!(PerpetualsError::MultisigAccountNotAuthorized);
        };

        // if not signed by this account return
        if !self.signed[signer_idx] {
            return Ok(());
        }

        // remove signature
        self.num_signed -= 1;
        self.signed[signer_idx] = false;

        Ok(())
    }

    /// Returns the array index of the provided signer
    pub fn get_signer_index(&self, signer: &Pubkey) -> Result<usize> {
        for i in 0..self.num_signers as usize {
            if &self.signers[i] == signer {
                return Ok(i);
            }
        }
        err!(PerpetualsError::MultisigAccountNotAuthorized)
    }

    /// Checks if provided account is one of multisig signers
    pub fn is_signer(&self, key: &Pubkey) -> Result<bool> {
        Ok(self.get_signer_index(key).is_ok())
    }
}
