//! Program state
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::UnixTimestamp,
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
};

/// Criteria for accepting a feature proposal
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub struct AcceptanceCriteria {
    /// The balance of the feature proposal's token account must be greater than this amount, and
    /// tallied before the deadline for the feature to be accepted.
    pub tokens_required: u64,

    /// If the required tokens are not tallied by this deadline then the proposal will expire.
    pub deadline: UnixTimestamp,
}

/// Contents of a Feature Proposal account
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub enum FeatureProposal {
    /// Default account state after creating it
    Uninitialized,
    /// Feature proposal is now pending
    Pending(AcceptanceCriteria),
    /// Feature proposal was accepted and the feature is now active
    Accepted {
        /// The balance of the feature proposal's token account at the time of activation.
        #[allow(dead_code)] // not dead code..
        tokens_upon_acceptance: u64,
    },
    /// Feature proposal was not accepted before the deadline
    Expired,
}
impl Sealed for FeatureProposal {}

impl Pack for FeatureProposal {
    const LEN: usize = 17; // see `test_get_packed_len()` for justification of "18"

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        dst[..data.len()].copy_from_slice(&data);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!(
                "Error: failed to deserialize feature proposal account: {}",
                err
            );
            ProgramError::InvalidAccountData
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_packed_len() {
        assert_eq!(
            FeatureProposal::get_packed_len(),
            solana_program::borsh::get_packed_len::<FeatureProposal>()
        );
    }

    #[test]
    fn test_serialize_bytes() {
        assert_eq!(FeatureProposal::Expired.try_to_vec().unwrap(), vec![3]);

        assert_eq!(
            FeatureProposal::Pending(AcceptanceCriteria {
                tokens_required: 0xdeadbeefdeadbeef,
                deadline: -1,
            })
            .try_to_vec()
            .unwrap(),
            vec![1, 239, 190, 173, 222, 239, 190, 173, 222, 255, 255, 255, 255, 255, 255, 255, 255],
        );
    }

    #[test]
    fn test_serialize_large_slice() {
        let mut dst = vec![0xff; 4];
        FeatureProposal::Expired.pack_into_slice(&mut dst);

        // Extra bytes (0xff) ignored
        assert_eq!(dst, vec![3, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn state_deserialize_invalid() {
        assert_eq!(
            FeatureProposal::unpack_from_slice(&[3]),
            Ok(FeatureProposal::Expired),
        );

        // Extra bytes (0xff) ignored...
        assert_eq!(
            FeatureProposal::unpack_from_slice(&[3, 0xff, 0xff, 0xff]),
            Ok(FeatureProposal::Expired),
        );

        assert_eq!(
            FeatureProposal::unpack_from_slice(&[4]),
            Err(ProgramError::InvalidAccountData),
        );
    }
}
