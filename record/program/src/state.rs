//! Program state
use {
    bytemuck::{Pod, Zeroable},
    solana_program::{program_pack::IsInitialized, pubkey::Pubkey},
};

/// Header type for recorded account data
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct RecordData {
    /// Struct version, allows for upgrades to the program
    pub version: u8,

    /// The account allowed to update the data
    pub authority: Pubkey,
}

impl RecordData {
    /// Version to fill in on new created accounts
    pub const CURRENT_VERSION: u8 = 1;

    /// Start of writable account data, after version and authority
    pub const WRITABLE_START_INDEX: usize = 33;
}

impl IsInitialized for RecordData {
    /// Is initialized
    fn is_initialized(&self) -> bool {
        self.version == Self::CURRENT_VERSION
    }
}

#[cfg(test)]
pub mod tests {
    use {
        super::*,
        solana_program::program_error::ProgramError,
        spl_pod::bytemuck::{pod_bytes_of, pod_from_bytes},
    };

    /// Version for tests
    pub const TEST_VERSION: u8 = 1;
    /// Pubkey for tests
    pub const TEST_PUBKEY: Pubkey = Pubkey::new_from_array([100; 32]);
    /// Bytes for tests
    pub const TEST_BYTES: [u8; 8] = [42; 8];
    /// RecordData for tests
    pub const TEST_RECORD_DATA: RecordData = RecordData {
        version: TEST_VERSION,
        authority: TEST_PUBKEY,
    };

    #[test]
    fn serialize_data() {
        let mut expected = vec![TEST_VERSION];
        expected.extend_from_slice(&TEST_PUBKEY.to_bytes());
        assert_eq!(pod_bytes_of(&TEST_RECORD_DATA), expected);
        assert_eq!(
            *pod_from_bytes::<RecordData>(&expected).unwrap(),
            TEST_RECORD_DATA,
        );
    }

    #[test]
    fn deserialize_invalid_slice() {
        let mut expected = vec![TEST_VERSION];
        expected.extend_from_slice(&TEST_PUBKEY.to_bytes());
        expected.extend_from_slice(&TEST_BYTES);
        let err: ProgramError = pod_from_bytes::<RecordData>(&expected).unwrap_err();
        assert_eq!(err, ProgramError::InvalidArgument);
    }
}
