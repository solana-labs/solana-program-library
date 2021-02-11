//! ZOL instruction

use crate::state::{EquivalenceProof, SolvencyProof, State, User};
use borsh::{BorshDeserialize, BorshSerialize};
use elgamal_ristretto::{ciphertext::Ciphertext, public::PublicKey};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

/// Instructions supported by the ZOL program.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum ZolInstruction {
    /// Initialize an empty vault account
    ///
    ///   0. `[writable]` Uninitialized ZOL account
    InitializeVault,

    /// Initialize an empty user account with an encryption key
    ///
    /// The `InitializeUser` instruction requires no signers and MUST be
    /// included within the same Transaction as the system program's
    /// `CreateAccount` instruction that creates the account being initialized.
    /// Otherwise another party can acquire ownership of the uninitialized
    /// account.
    ///
    ///   0. `[writable]` Uninitialized ZOL account
    InitializeUser { encryption_pubkey: PublicKey },

    /// Mint ZOL by depositing SOL into a vault. `amount` will be
    /// deposited into the vault and `amount` will be added to
    /// the user account.
    ///
    ///   0. `[signer]` Sender account holding SOL
    ///   1. `[writable]` Vault account
    ///   2. `[writable]` Uninitialized ZOL account
    Deposit { amount: u64 },

    /// Withdraw SOL from a user account.
    ///
    ///   0. `[signer]` Sender ZOL account
    ///   1. `[writable]` Vault account
    ///   2. `[writable]` Recipient System account
    Withdraw {
        amount: u64,
        solvency_proof: SolvencyProof,
    },

    /// Transfer ZOL between two user accounts.
    ///
    ///   0. `[signer]` Sender ZOL account
    ///   1. `[writable]` Recipient ZOL account
    Transfer {
        sender_amount: Ciphertext,
        recipient_amount: Ciphertext,
        solvency_proof: SolvencyProof,
        equivalence_proof: EquivalenceProof,
    },
}

impl ZolInstruction {
    pub fn serialize(&self) -> Result<Vec<u8>, ProgramError> {
        self.try_to_vec()
            .map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub(crate) fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidInstructionData)
    }
}

/// Return an `InitializeVault` instruction.
fn initialize_vault(program_id: &Pubkey, vault_pubkey: &Pubkey) -> Instruction {
    let data = ZolInstruction::InitializeVault;
    let accounts = vec![AccountMeta::new(*vault_pubkey, false)];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return an `InitializeUser` instruction.
fn initialize_user(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    encryption_pubkey: &PublicKey,
) -> Instruction {
    let data = ZolInstruction::InitializeUser {
        encryption_pubkey: *encryption_pubkey,
    };
    let accounts = vec![AccountMeta::new(*user_pubkey, false)];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return two instructions that create and initialize a vault account.
pub fn create_vault(
    program_id: &Pubkey,
    from_account_pubkey: &Pubkey,
    vault_pubkey: &Pubkey,
    lamports: u64,
) -> Vec<Instruction> {
    let space = State::Vault.packed_len() as u64;
    vec![
        system_instruction::create_account(
            from_account_pubkey,
            vault_pubkey,
            lamports,
            space,
            program_id,
        ),
        initialize_vault(program_id, vault_pubkey),
    ]
}

/// Return two instructions that create and initialize a user account.
pub fn create_user(
    program_id: &Pubkey,
    from_account_pubkey: &Pubkey,
    user_pubkey: &Pubkey,
    encryption_pubkey: &PublicKey,
    lamports: u64,
) -> Vec<Instruction> {
    let space = State::User(User::new(*encryption_pubkey)).packed_len() as u64;
    vec![
        system_instruction::create_account(
            from_account_pubkey,
            user_pubkey,
            lamports,
            space,
            program_id,
        ),
        initialize_user(program_id, user_pubkey, encryption_pubkey),
    ]
}
