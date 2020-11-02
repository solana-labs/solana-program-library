//! Associated program instructions

use speedy::{Readable, Writable};

/// The Create instruction will allocate space for a new associated account.
///
/// Accounts expected by this instruction:
///
///   0. `[writeable]` Associated address
///   1. `[]` Primary address of the associated account (typically a system account)
///   2. `[]` Address of program that will own the associated account
///   3. `[writeable,signer]` Funding account (must be a system account)
///   4. `[]` System program
///   5. ..5+N `[]` N optional additional associated address seeds
///
#[repr(C)]
#[derive(Clone, Debug, Readable, Writable)]
pub struct InstructionData {
    /// Ensure the new account contains this amount of lamports
    pub lamports: u64,

    /// Number of bytes of memory to allocate
    pub space: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack() {
        assert_eq!(
            InstructionData {
                lamports: 0xdead,
                space: 0xbeef,
            }
            .write_to_vec()
            .unwrap(),
            vec![0xad, 0xde, 0, 0, 0, 0, 0, 0, 0xef, 0xbe, 0, 0, 0, 0, 0, 0]
        );
    }
}
