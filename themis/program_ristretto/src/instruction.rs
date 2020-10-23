//! Instruction types

use crate::state::{Policies, User};
use borsh::{BorshDeserialize, BorshSerialize};
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use elgamal_ristretto::public::PublicKey;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

/// Instructions supported by the Themis program.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum ThemisInstruction {
    /// Initialize a new user account
    ///
    /// The `InitializeUserAccount` instruction requires no signers and MUST be included within
    /// the same Transaction as the system program's `CreateInstruction` that creates the account
    /// being initialized.  Otherwise another party can acquire ownership of the uninitialized account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The account to initialize.
    InitializeUserAccount {
        /// Public key for all encrypted interations
        public_key: PublicKey,
    },

    /// Initialize a new policies account
    ///
    /// The `InitializePoliciesAccount` instruction requires no signers and MUST be included within
    /// the same Transaction as the system program's `CreateInstruction` that creates the account
    /// being initialized.  Otherwise another party can acquire ownership of the uninitialized account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The account to initialize.
    InitializePoliciesAccount {
        /// Number of policies to be added
        num_scalars: u8,
    },

    /// Store policies
    ///
    /// The `StorePolices` instruction is used to set individual policies.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]` The policies account.
    StorePolicies {
        /// Policies to be added
        scalars: Vec<(u8, Scalar)>,
    },

    /// Calculate aggregate. The length of the `input` vector must equal the
    /// number of policies.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]`  The user account
    ///   1. `[]`  The policies account
    SubmitInteractions {
        /// Encrypted interactions
        encrypted_interactions: Vec<(u8, (RistrettoPoint, RistrettoPoint))>,
    },

    /// Submit proof decryption
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]`  The user account
    SubmitProofDecryption {
        /// plaintext
        plaintext: RistrettoPoint,

        /// (announcement_g, announcement_ctx)
        announcement: Box<(RistrettoPoint, RistrettoPoint)>,

        /// response
        response: Scalar,
    },

    /// Request a payment
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]`  The user account
    RequestPayment {
        /// Encrypted aggregate
        encrypted_aggregate: Box<(RistrettoPoint, RistrettoPoint)>,

        /// Decrypted aggregate
        decrypted_aggregate: RistrettoPoint,

        /// Proof correct decryption
        proof_correct_decryption: RistrettoPoint,
    },
}

impl ThemisInstruction {
    pub fn serialize(&self) -> Result<Vec<u8>, ProgramError> {
        self.try_to_vec()
            .map_err(|_| ProgramError::AccountDataTooSmall)
    }

    pub(crate) fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidInstructionData)
    }
}

/// Return an `InitializeUserAccount` instruction.
fn initialize_user_account(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    public_key: PublicKey,
) -> Instruction {
    let data = ThemisInstruction::InitializeUserAccount { public_key };

    let accounts = vec![AccountMeta::new(*user_pubkey, false)];

    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return two instructions that create and initialize a user account.
pub fn create_user_account(
    program_id: &Pubkey,
    from: &Pubkey,
    user_pubkey: &Pubkey,
    lamports: u64,
    public_key: PublicKey,
) -> Vec<Instruction> {
    let space = User::default().try_to_vec().unwrap().len() as u64;
    vec![
        system_instruction::create_account(from, user_pubkey, lamports, space, program_id),
        initialize_user_account(program_id, user_pubkey, public_key),
    ]
}

/// Return an `InitializePoliciesAccount` instruction.
fn initialize_policies_account(
    program_id: &Pubkey,
    policies_pubkey: &Pubkey,
    num_scalars: u8,
) -> Instruction {
    let data = ThemisInstruction::InitializePoliciesAccount { num_scalars };
    let accounts = vec![AccountMeta::new(*policies_pubkey, false)];
    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return two instructions that create and initialize a policies account.
pub fn create_policies_account(
    program_id: &Pubkey,
    from: &Pubkey,
    policies_pubkey: &Pubkey,
    lamports: u64,
    num_scalars: u8,
) -> Vec<Instruction> {
    let space = Policies::new(num_scalars).try_to_vec().unwrap().len() as u64;
    vec![
        system_instruction::create_account(from, policies_pubkey, lamports, space, program_id),
        initialize_policies_account(program_id, policies_pubkey, num_scalars),
    ]
}

/// Return an `InitializePoliciesAccount` instruction.
pub fn store_policies(
    program_id: &Pubkey,
    policies_pubkey: &Pubkey,
    scalars: Vec<(u8, Scalar)>,
) -> Instruction {
    let data = ThemisInstruction::StorePolicies { scalars };
    let accounts = vec![AccountMeta::new(*policies_pubkey, true)];
    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return a `SubmitInteractions` instruction.
pub fn submit_interactions(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    policies_pubkey: &Pubkey,
    encrypted_interactions: Vec<(u8, (RistrettoPoint, RistrettoPoint))>,
) -> Instruction {
    let data = ThemisInstruction::SubmitInteractions {
        encrypted_interactions,
    };
    let accounts = vec![
        AccountMeta::new(*user_pubkey, true),
        AccountMeta::new_readonly(*policies_pubkey, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return a `SubmitProofDecryption` instruction.
pub fn submit_proof_decryption(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    plaintext: RistrettoPoint,
    announcement_g: RistrettoPoint,
    announcement_ctx: RistrettoPoint,
    response: Scalar,
) -> Instruction {
    let data = ThemisInstruction::SubmitProofDecryption {
        plaintext,
        announcement: Box::new((announcement_g, announcement_ctx)),
        response,
    };
    let accounts = vec![AccountMeta::new(*user_pubkey, true)];
    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}

/// Return a `RequestPayment` instruction.
pub fn request_payment(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    encrypted_aggregate: (RistrettoPoint, RistrettoPoint),
    decrypted_aggregate: RistrettoPoint,
    proof_correct_decryption: RistrettoPoint,
) -> Instruction {
    let data = ThemisInstruction::RequestPayment {
        encrypted_aggregate: Box::new(encrypted_aggregate),
        decrypted_aggregate,
        proof_correct_decryption,
    };
    let accounts = vec![AccountMeta::new(*user_pubkey, true)];
    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
}
