//! Instruction types

use curve25519_dalek::{
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};
use elgamal_ristretto::public::PublicKey;
use serde::{Deserialize, Serialize};

/// Instructions supported by the Themis program.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
        policies: Vec<Scalar>,
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
        encrypted_interactions: Vec<(RistrettoPoint, RistrettoPoint)>,

        /// Public key for all encrypted interations
        public_key: PublicKey,
    },

    /// Submit proof decryption
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The user account
    SubmitProofDecryption {
        /// plaintext
        plaintext: RistrettoPoint,

        /// announcement_g
        announcement_g: CompressedRistretto,

        /// announcement_ctx
        announcement_ctx: CompressedRistretto,

        /// response
        response: Scalar,
    },

    /// Request a payment
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The user account
    RequestPayment {
        /// Encrypted aggregate
        encrypted_aggregate: (RistrettoPoint, RistrettoPoint),

        /// Decrypted aggregate
        decrypted_aggregate: RistrettoPoint,

        /// Proof correct decryption
        proof_correct_decryption: RistrettoPoint,
    },
}
