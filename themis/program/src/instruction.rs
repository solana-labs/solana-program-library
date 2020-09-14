//! Instruction types

use crate::error::ThemisError;
use bn::arith::U256;
use solana_sdk::program_error::ProgramError;

/// Instructions supported by the Themis program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
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
    InitializeUserAccount,

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
        policies: Vec<U256>,
    },

    /// Calculate aggregate. The length of the `input` vector must equal the
    /// number of policies.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The user account
    ///   1. `[writable]`  The policies account
    CalculateAggregate {
        /// Encrypted interactions
        encrypted_interactions: Vec<[U256; 4]>,

        /// Public key for all encrypted interations
        public_key: [U256; 2],
    },

    /// Submit proof decryption
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The user account
    SubmitProofDecryption {
        /// plaintext
        plaintext: [U256; 2],

        /// announcement_g
        announcement_g: [U256; 2],

        /// announcement_ctx
        announcement_ctx: [U256; 2],

        /// response
        response: U256,
    },

    /// Request a payment
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The user account
    RequestPayment {
        /// Encrypted aggregate
        encrypted_aggregate: [U256; 4],

        /// Decrypted aggregate
        decrypted_aggregate: [U256; 2],

        /// Proof correct decryption
        proof_correct_decryption: [U256; 2],
    },
}

fn unpack_u256(input: &[u8]) -> Result<(U256, &[u8]), ProgramError> {
    use ThemisError::InvalidInstruction;

    if input.len() >= 32 {
        let (u256_slice, rest) = input.split_at(32);
        let u256 = U256::from_slice(u256_slice).map_err(|_| InvalidInstruction)?;
        Ok((u256, rest))
    } else {
        Err(InvalidInstruction.into())
    }
}

impl ThemisInstruction {
    /// Unpacks a byte buffer into a [ThemisInstruction](enum.ThemisInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use ThemisError::InvalidInstruction;

        let (&tag, _rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => ThemisInstruction::InitializeUserAccount,
            1 => ThemisInstruction::InitializePoliciesAccount { policies: vec![] },
            2 => {
                let (pk1, input) = unpack_u256(input)?;
                let (pk2, _input) = unpack_u256(input)?;
                if !input.is_empty() {
                    return Err(InvalidInstruction.into());
                }
                ThemisInstruction::CalculateAggregate {
                    encrypted_interactions: vec![],
                    public_key: [pk1, pk2],
                }
            },
            3 => ThemisInstruction::SubmitProofDecryption {
                plaintext: [U256::zero(), U256::zero()],
                announcement_g: [U256::zero(), U256::zero()],
                announcement_ctx: [U256::zero(), U256::zero()],
                response: U256::zero(),
            },
            4 => ThemisInstruction::RequestPayment {
                encrypted_aggregate: [U256::zero(), U256::zero(), U256::zero(), U256::zero()],
                decrypted_aggregate: [U256::zero(), U256::zero()],
                proof_correct_decryption: [U256::zero(), U256::zero()],
            },
            _ => return Err(InvalidInstruction.into()),
        })
    }
}
