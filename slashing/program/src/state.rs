//! Program state
use {
    crate::error::SlashingError,
    bytemuck::{Pod, Zeroable},
    solana_program::{program_pack::IsInitialized, pubkey::Pubkey},
};

const PACKET_DATA_SIZE: usize = 1232;

/// Types of slashing proofs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofType {
    /// Proof consisting of 2 shreds signed by the leader indicating the leader
    /// submitted a duplicate block.
    DuplicateBlockProof,
    /// Invalid proof type
    InvalidType,
}

impl ProofType {
    /// Size of the proof account to create in order to hold the proof data
    /// header and contents
    pub fn proof_account_length(&self) -> Result<usize, SlashingError> {
        match self {
            // Duplicate block proof consists of 2 shreds
            Self::DuplicateBlockProof => Ok(2 * PACKET_DATA_SIZE + ProofData::WRITABLE_START_INDEX),
            Self::InvalidType => Err(SlashingError::InvalidProofType),
        }
    }
}

impl From<ProofType> for u8 {
    fn from(value: ProofType) -> u8 {
        match value {
            ProofType::DuplicateBlockProof => 0,
            ProofType::InvalidType => u8::MAX,
        }
    }
}

impl From<u8> for ProofType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::DuplicateBlockProof,
            _ => Self::InvalidType,
        }
    }
}

/// Header type for proof data
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ProofData {
    /// Struct version, allows for upgrades to the program
    pub version: u8,

    /// Type of proof, determines the size
    pub proof_type: u8,

    /// The account allowed to update the data
    pub authority: Pubkey,
}

impl ProofData {
    /// Version to fill in on new created accounts
    pub const CURRENT_VERSION: u8 = 1;

    /// Start of writable account data, after version, proof type and authority
    pub const WRITABLE_START_INDEX: usize = 34;
}

impl IsInitialized for ProofData {
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
    /// Proof type for tests
    pub const TEST_PROOF_TYPE: u8 = 0;
    /// Pubkey for tests
    pub const TEST_PUBKEY: Pubkey = Pubkey::new_from_array([100; 32]);
    /// Bytes for tests
    pub const TEST_BYTES: [u8; 8] = [42; 8];
    /// ProofData for tests
    pub const TEST_PROOF_DATA: ProofData = ProofData {
        version: TEST_VERSION,
        proof_type: TEST_PROOF_TYPE,
        authority: TEST_PUBKEY,
    };

    #[test]
    fn serialize_data() {
        let mut expected = vec![TEST_VERSION, TEST_PROOF_TYPE];
        expected.extend_from_slice(&TEST_PUBKEY.to_bytes());
        assert_eq!(pod_bytes_of(&TEST_PROOF_DATA), expected);
        assert_eq!(
            *pod_from_bytes::<ProofData>(&expected).unwrap(),
            TEST_PROOF_DATA,
        );
    }

    #[test]
    fn deserialize_invalid_slice() {
        let mut expected = vec![TEST_VERSION];
        expected.extend_from_slice(&TEST_PUBKEY.to_bytes());
        expected.extend_from_slice(&TEST_BYTES);
        let err: ProgramError = pod_from_bytes::<ProofData>(&expected).unwrap_err();
        assert_eq!(err, ProgramError::InvalidArgument);
    }
}
