//! Interface state types

use {
    crate::error::TokenGroupError,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    spl_discriminator::SplDiscriminate,
    spl_pod::{error::PodSliceError, optional_keys::OptionalNonZeroPubkey},
    spl_type_length_value::{variable_len_pack::VariableLenPack, SplBorshVariableLenPack},
};

/// Trait defining a `Group` context
pub trait SplTokenGroup:
    BorshDeserialize + BorshSerialize + Clone + SplDiscriminate + VariableLenPack
{
}

/// Data struct for a `Group`
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    SplBorshVariableLenPack,
    SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_group_interface:group")]
pub struct Group<G>
where
    G: SplTokenGroup,
{
    /// The authority that can sign to update the group
    pub update_authority: OptionalNonZeroPubkey,
    /// The current number of group members
    pub size: u64,
    /// The maximum number of group members
    pub max_size: Option<u64>,
    /// Additional state
    pub meta: Option<G>,
}

impl<G> Group<G>
where
    G: SplTokenGroup,
{
    /// Creates a new `Group` state
    pub fn new(
        update_authority: OptionalNonZeroPubkey,
        max_size: Option<u64>,
        meta: Option<G>,
    ) -> Self {
        Self {
            update_authority,
            size: 0,
            max_size,
            meta,
        }
    }

    /// Updates the max size for a group
    pub fn update_max_size(&mut self, max_size: Option<u64>) -> Result<(), ProgramError> {
        // The new max size cannot be less than the current size
        if let Some(new_max_size) = max_size {
            if new_max_size < self.size {
                return Err(TokenGroupError::SizeExceedsNewMaxSize.into());
            }
        }
        self.max_size = max_size;
        Ok(())
    }

    /// Increment the size for a group, returning the new size
    pub fn increment_size(&mut self) -> Result<u64, ProgramError> {
        // The new size cannot be greater than the max size
        let new_size = self
            .size
            .checked_add(1)
            .ok_or::<ProgramError>(PodSliceError::CalculationFailure.into())?;
        if let Some(max_size) = self.max_size {
            if new_size > max_size {
                return Err(TokenGroupError::SizeExceedsMaxSize.into());
            }
        }
        self.size = new_size;
        Ok(self.size)
    }
}

/// Data struct for a `Member` of a `Group`
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    SplBorshVariableLenPack,
    SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_group_interface:member")]
pub struct Member {
    /// The pubkey of the `Group`
    pub group: Pubkey,
    /// The member number
    pub member_number: u64,
}
impl Member {
    /// Creates a new `Member` state
    pub fn new(group: Pubkey, member_number: u64) -> Self {
        Self {
            group,
            member_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::NAMESPACE,
        solana_program::{borsh::get_instance_packed_len, hash},
        spl_discriminator::ArrayDiscriminator,
        spl_type_length_value::{
            error::TlvError,
            state::{TlvState, TlvStateBorrowed, TlvStateMut},
        },
    };

    #[derive(
        Clone,
        Debug,
        Default,
        PartialEq,
        BorshSerialize,
        BorshDeserialize,
        SplDiscriminate,
        SplBorshVariableLenPack,
    )]
    #[discriminator_hash_input("mock_group")]
    struct MockGroup {
        pub data: u64,
    }
    impl SplTokenGroup for MockGroup {}

    #[test]
    fn discriminators() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:group").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(Group::<MockGroup>::SPL_DISCRIMINATOR, discriminator);

        let preimage = hash::hashv(&[format!("{NAMESPACE}:member").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(Member::SPL_DISCRIMINATOR, discriminator);
    }

    #[test]
    fn tlv_state_pack() {
        // Make sure we can pack more than one instance of each type
        let group = Group {
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 10,
            max_size: Some(20),
            meta: Some(MockGroup { data: 30 }),
        };
        let group_instance_size = get_instance_packed_len(&group).unwrap();

        let member_data = Member {
            group: Pubkey::new_unique(),
            member_number: 0,
        };
        let member_instance_size = get_instance_packed_len(&member_data).unwrap();

        let account_size = TlvStateBorrowed::get_base_len()
            + get_instance_packed_len(&group).unwrap()
            + TlvStateBorrowed::get_base_len()
            + get_instance_packed_len(&member_data).unwrap();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        state
            .alloc::<Group<MockGroup>>(group_instance_size, false)
            .unwrap();
        state.pack_first_variable_len_value(&group).unwrap();

        state.alloc::<Member>(member_instance_size, false).unwrap();
        state.pack_first_variable_len_value(&member_data).unwrap();

        assert_eq!(
            state
                .get_first_variable_len_value::<Group<MockGroup>>()
                .unwrap(),
            group
        );
        assert_eq!(
            state.get_first_variable_len_value::<Member>().unwrap(),
            member_data
        );

        // But we don't want to be able to pack two of the same

        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        state
            .alloc::<Group<MockGroup>>(group_instance_size, false)
            .unwrap();
        state.pack_first_variable_len_value(&group).unwrap();

        assert_eq!(
            state
                .alloc::<Group<MockGroup>>(group_instance_size, false)
                .unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );

        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        state.alloc::<Member>(member_instance_size, false).unwrap();
        state.pack_first_variable_len_value(&member_data).unwrap();

        assert_eq!(
            state
                .alloc::<Member>(member_instance_size, false)
                .unwrap_err(),
            TlvError::TypeAlreadyExists.into(),
        );
    }

    #[test]
    fn update_max_size() {
        // Test with a `Some` max size
        let max_size = Some(10);
        let mut group = Group {
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 0,
            max_size,
            meta: Some(MockGroup { data: 30 }),
        };

        let new_max_size = Some(30);
        group.update_max_size(new_max_size).unwrap();
        assert_eq!(group.max_size, new_max_size);

        // Change the current size to 30
        group.size = 30;

        // Try to set the max size to 20, which is less than the current size
        let new_max_size = Some(20);
        assert_eq!(
            group.update_max_size(new_max_size),
            Err(ProgramError::from(TokenGroupError::SizeExceedsNewMaxSize))
        );

        // Test with a `None` max size
        let max_size = None;
        let mut group = Group {
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 0,
            max_size,
            meta: Some(MockGroup { data: 30 }),
        };

        let new_max_size = Some(30);
        group.update_max_size(new_max_size).unwrap();
        assert_eq!(group.max_size, new_max_size);
    }

    #[test]
    fn increment_current_size() {
        let mut group = Group {
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 0,
            max_size: Some(1),
            meta: Some(MockGroup { data: 30 }),
        };

        group.increment_size().unwrap();
        assert_eq!(group.size, 1);

        // Try to increase the current size to 2, which is greater than the max size
        assert_eq!(
            group.increment_size(),
            Err(ProgramError::from(TokenGroupError::SizeExceedsMaxSize))
        );

        // Test with a `None` max size
        let mut group = Group {
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 0,
            max_size: None,
            meta: Some(MockGroup { data: 30 }),
        };

        group.increment_size().unwrap();
        assert_eq!(group.size, 1);
    }
}
