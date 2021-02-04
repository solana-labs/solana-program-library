//! Program state
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
};

pub const DATA_SIZE: usize = 1_000;

/// Criteria for accepting a feature proposal
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct Document {
    /// Struct version, allows for upgrades to the program
    pub version: u8,

    /// The account allowed to update the document
    pub owner: Pubkey,

    /// The data contained by the account, could be anything or serializable
    pub data: [u8; DATA_SIZE],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_serialize() {
        let document = Document {
            version: 1,
            owner: Pubkey::new_unique(),
            data: [0; DATA_SIZE],
        };
        let encoded = document.try_to_vec().unwrap();
        assert_eq!(document.try_to_vec().unwrap(), vec![3]);
    }

    #[test]
    fn document_deserialize() {
        // TODO
    }
}
