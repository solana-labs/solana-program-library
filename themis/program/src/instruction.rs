//! Instruction types

use bn::arith::U256;

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
