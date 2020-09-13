//! Instruction types

use bn::arith::U256;

/// Instructions supported by the Themis program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum ThemisInstruction {
    /// Initialize a new client account
    ///
    /// The `InitializeClientAccount` instruction requires no signers and MUST be included within
    /// the same Transaction as the system program's `CreateInstruction` that creates the account
    /// being initialized.  Otherwise another party can acquire ownership of the uninitialized account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The account to initialize.
    InitializeClientAccount,

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
    ///   0. `[writable]`  The client account
    ///   1. `[writable]`  The policies account
    CalculateAggregate {
        /// Encrypted interactions
        input: Vec<[U256; 4]>,

        /// Public key for all encrypted interations
        public_key: [U256; 2],
    },

    /// Submit proof decryption
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The client account
    SubmitProofDecryption {
        /// plaintext, announcment_g, announcment_ctx, response
        input: [U256; 7],
    },

    /// Request a payment
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The client account
    RequestPayment {
        /// Encrypted aggregate
        encrypted_aggregate: [U256; 4],

        /// Decrypted aggregate
        decrypted_aggregate: [U256; 2],

        /// Proof correct decryption
        proof_correct_decryption: [U256; 2],
    },
}
