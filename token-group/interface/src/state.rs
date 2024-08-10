//! Interface state types

use {
    crate::error::TokenGroupError,
    bytemuck::{Pod, Zeroable},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    spl_discriminator::SplDiscriminate,
    spl_pod::{error::PodSliceError, optional_keys::OptionalNonZeroPubkey, primitives::PodU64},
};

/// Data struct for a `TokenGroup`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:group")]
pub struct TokenGroup {
    /// The authority that can sign to update the group
    pub update_authority: OptionalNonZeroPubkey,
    /// The associated mint, used to counter spoofing to be sure that group
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The current number of group members
    pub size: PodU64,
    /// The maximum number of group members
    pub max_size: PodU64,
}

impl TokenGroup {
    /// Creates a new `TokenGroup` state
    pub fn new(mint: &Pubkey, update_authority: OptionalNonZeroPubkey, max_size: u64) -> Self {
        Self {
            mint: *mint,
            update_authority,
            size: PodU64::default(), // [0, 0, 0, 0, 0, 0, 0, 0]
            max_size: max_size.into(),
        }
    }

    /// Updates the max size for a group
    pub fn update_max_size(&mut self, new_max_size: u64) -> Result<(), ProgramError> {
        // The new max size cannot be less than the current size
        if new_max_size < u64::from(self.size) {
            return Err(TokenGroupError::SizeExceedsNewMaxSize.into());
        }
        self.max_size = new_max_size.into();
        Ok(())
    }

    /// Increment the size for a group, returning the new size
    pub fn increment_size(&mut self) -> Result<u64, ProgramError> {
        // The new size cannot be greater than the max size
        let new_size = u64::from(self.size)
            .checked_add(1)
            .ok_or::<ProgramError>(PodSliceError::CalculationFailure.into())?;
        if new_size > u64::from(self.max_size) {
            return Err(TokenGroupError::SizeExceedsMaxSize.into());
        }
        self.size = new_size.into();
        Ok(new_size)
    }
}

/// Data struct for a `TokenGroupMember`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:member")]
pub struct TokenGroupMember {
    /// The associated mint, used to counter spoofing to be sure that member
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The pubkey of the `TokenGroup`
    pub group: Pubkey,
    /// The member number
    pub member_number: PodU64,
}
impl TokenGroupMember {
    /// Creates a new `TokenGroupMember` state
    pub fn new(mint: &Pubkey, group: &Pubkey, member_number: u64) -> Self {
        Self {
            mint: *mint,
            group: *group,
            member_number: member_number.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::NAMESPACE,
        solana_program::hash,
        spl_discriminator::ArrayDiscriminator,
        spl_type_length_value::state::{TlvState, TlvStateBorrowed, TlvStateMut},
        std::mem::size_of,
    };

    #[test]
    fn discriminators() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:group").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(TokenGroup::SPL_DISCRIMINATOR, discriminator);

        let preimage = hash::hashv(&[format!("{NAMESPACE}:member").as_bytes()]);
        let discriminator =
            ArrayDiscriminator::try_from(&preimage.as_ref()[..ArrayDiscriminator::LENGTH]).unwrap();
        assert_eq!(TokenGroupMember::SPL_DISCRIMINATOR, discriminator);
    }

    #[test]
    fn tlv_state_pack() {
        // Make sure we can pack more than one instance of each type
        let group = TokenGroup {
            mint: Pubkey::new_unique(),
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 10.into(),
            max_size: 20.into(),
        };

        let member = TokenGroupMember {
            mint: Pubkey::new_unique(),
            group: Pubkey::new_unique(),
            member_number: 0.into(),
        };

        let account_size = TlvStateBorrowed::get_base_len()
            + size_of::<TokenGroup>()
            + TlvStateBorrowed::get_base_len()
            + size_of::<TokenGroupMember>();
        let mut buffer = vec![0; account_size];
        let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

        let group_data = state.init_value::<TokenGroup>(false).unwrap().0;
        *group_data = group;

        let member_data = state.init_value::<TokenGroupMember>(false).unwrap().0;
        *member_data = member;

        assert_eq!(state.get_first_value::<TokenGroup>().unwrap(), &group);
        assert_eq!(
            state.get_first_value::<TokenGroupMember>().unwrap(),
            &member
        );
    }

    #[test]
    fn update_max_size() {
        // Test with a `Some` max size
        let max_size = 10;
        let mut group = TokenGroup {
            mint: Pubkey::new_unique(),
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 0.into(),
            max_size: max_size.into(),
        };

        let new_max_size = 30;
        group.update_max_size(new_max_size).unwrap();
        assert_eq!(u64::from(group.max_size), new_max_size);

        // Change the current size to 30
        group.size = 30.into();

        // Try to set the max size to 20, which is less than the current size
        let new_max_size = 20;
        assert_eq!(
            group.update_max_size(new_max_size),
            Err(ProgramError::from(TokenGroupError::SizeExceedsNewMaxSize))
        );

        let new_max_size = 30;
        group.update_max_size(new_max_size).unwrap();
        assert_eq!(u64::from(group.max_size), new_max_size);
    }

    #[test]
    fn increment_current_size() {
        let mut group = TokenGroup {
            mint: Pubkey::new_unique(),
            update_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap(),
            size: 0.into(),
            max_size: 1.into(),
        };

        group.increment_size().unwrap();
        assert_eq!(u64::from(group.size), 1);

        // Try to increase the current size to 2, which is greater than the max size
        assert_eq!(
            group.increment_size(),
            Err(ProgramError::from(TokenGroupError::SizeExceedsMaxSize))
        );
    }
}
