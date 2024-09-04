//! Instruction types

use {
    bytemuck::{Pod, Zeroable},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_pod::{
        bytemuck::{pod_bytes_of, pod_from_bytes},
        optional_keys::OptionalNonZeroPubkey,
        primitives::PodU64,
    },
};

/// Instruction data for initializing a new `Group`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:initialize_token_group")]
pub struct InitializeGroup {
    /// Update authority for the group
    pub update_authority: OptionalNonZeroPubkey,
    /// The maximum number of group members
    pub max_size: PodU64,
}

/// Instruction data for updating the max size of a `Group`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:update_group_max_size")]
pub struct UpdateGroupMaxSize {
    /// New max size for the group
    pub max_size: PodU64,
}

/// Instruction data for updating the authority of a `Group`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:update_authority")]
pub struct UpdateGroupAuthority {
    /// New authority for the group, or unset if `None`
    pub new_authority: OptionalNonZeroPubkey,
}

/// Instruction data for initializing a new `Member` of a `Group`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:initialize_member")]
pub struct InitializeMember;

/// All instructions that must be implemented in the SPL Token Group Interface
#[derive(Clone, Debug, PartialEq)]
pub enum TokenGroupInstruction {
    /// Initialize a new `Group`
    ///
    /// Assumes one has already initialized a mint for the
    /// group.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Group
    ///   1. `[]`   Mint
    ///   2. `[s]`  Mint authority
    InitializeGroup(InitializeGroup),

    /// Update the max size of a `Group`
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Group
    ///   1. `[s]`  Update authority
    UpdateGroupMaxSize(UpdateGroupMaxSize),

    /// Update the authority of a `Group`
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Group
    ///   1. `[s]`  Current update authority
    UpdateGroupAuthority(UpdateGroupAuthority),

    /// Initialize a new `Member` of a `Group`
    ///
    /// Assumes the `Group` has already been initialized,
    /// as well as the mint for the member.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Member
    ///   1. `[]`   Member mint
    ///   1. `[s]`  Member mint authority
    ///   2. `[w]`  Group
    ///   3. `[s]`  Group update authority
    InitializeMember(InitializeMember),
}
impl TokenGroupInstruction {
    /// Unpacks a byte buffer into a `TokenGroupInstruction`
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(ArrayDiscriminator::LENGTH);
        Ok(match discriminator {
            InitializeGroup::SPL_DISCRIMINATOR_SLICE => {
                let data = pod_from_bytes::<InitializeGroup>(rest)?;
                Self::InitializeGroup(*data)
            }
            UpdateGroupMaxSize::SPL_DISCRIMINATOR_SLICE => {
                let data = pod_from_bytes::<UpdateGroupMaxSize>(rest)?;
                Self::UpdateGroupMaxSize(*data)
            }
            UpdateGroupAuthority::SPL_DISCRIMINATOR_SLICE => {
                let data = pod_from_bytes::<UpdateGroupAuthority>(rest)?;
                Self::UpdateGroupAuthority(*data)
            }
            InitializeMember::SPL_DISCRIMINATOR_SLICE => {
                let data = pod_from_bytes::<InitializeMember>(rest)?;
                Self::InitializeMember(*data)
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a `TokenGroupInstruction` into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::InitializeGroup(data) => {
                buf.extend_from_slice(InitializeGroup::SPL_DISCRIMINATOR_SLICE);
                buf.extend_from_slice(pod_bytes_of(data));
            }
            Self::UpdateGroupMaxSize(data) => {
                buf.extend_from_slice(UpdateGroupMaxSize::SPL_DISCRIMINATOR_SLICE);
                buf.extend_from_slice(pod_bytes_of(data));
            }
            Self::UpdateGroupAuthority(data) => {
                buf.extend_from_slice(UpdateGroupAuthority::SPL_DISCRIMINATOR_SLICE);
                buf.extend_from_slice(pod_bytes_of(data));
            }
            Self::InitializeMember(data) => {
                buf.extend_from_slice(InitializeMember::SPL_DISCRIMINATOR_SLICE);
                buf.extend_from_slice(pod_bytes_of(data));
            }
        };
        buf
    }
}

/// Creates a `InitializeGroup` instruction
pub fn initialize_group(
    program_id: &Pubkey,
    group: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    update_authority: Option<Pubkey>,
    max_size: u64,
) -> Instruction {
    let update_authority = OptionalNonZeroPubkey::try_from(update_authority)
        .expect("Failed to deserialize `Option<Pubkey>`");
    let data = TokenGroupInstruction::InitializeGroup(InitializeGroup {
        update_authority,
        max_size: max_size.into(),
    })
    .pack();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*group, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*mint_authority, true),
        ],
        data,
    }
}

/// Creates a `UpdateGroupMaxSize` instruction
pub fn update_group_max_size(
    program_id: &Pubkey,
    group: &Pubkey,
    update_authority: &Pubkey,
    max_size: u64,
) -> Instruction {
    let data = TokenGroupInstruction::UpdateGroupMaxSize(UpdateGroupMaxSize {
        max_size: max_size.into(),
    })
    .pack();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*group, false),
            AccountMeta::new_readonly(*update_authority, true),
        ],
        data,
    }
}

/// Creates a `UpdateGroupAuthority` instruction
pub fn update_group_authority(
    program_id: &Pubkey,
    group: &Pubkey,
    current_authority: &Pubkey,
    new_authority: Option<Pubkey>,
) -> Instruction {
    let new_authority = OptionalNonZeroPubkey::try_from(new_authority)
        .expect("Failed to deserialize `Option<Pubkey>`");
    let data =
        TokenGroupInstruction::UpdateGroupAuthority(UpdateGroupAuthority { new_authority }).pack();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*group, false),
            AccountMeta::new_readonly(*current_authority, true),
        ],
        data,
    }
}

/// Creates a `InitializeMember` instruction
#[allow(clippy::too_many_arguments)]
pub fn initialize_member(
    program_id: &Pubkey,
    member: &Pubkey,
    member_mint: &Pubkey,
    member_mint_authority: &Pubkey,
    group: &Pubkey,
    group_update_authority: &Pubkey,
) -> Instruction {
    let data = TokenGroupInstruction::InitializeMember(InitializeMember {}).pack();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*member, false),
            AccountMeta::new_readonly(*member_mint, false),
            AccountMeta::new_readonly(*member_mint_authority, true),
            AccountMeta::new(*group, false),
            AccountMeta::new_readonly(*group_update_authority, true),
        ],
        data,
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, SplDiscriminate)]
    #[discriminator_hash_input("mock_group")]
    struct MockGroup;

    fn instruction_pack_unpack<I>(instruction: TokenGroupInstruction, discriminator: &[u8], data: I)
    where
        I: core::fmt::Debug + PartialEq + Pod + Zeroable + SplDiscriminate,
    {
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.extend_from_slice(pod_bytes_of(&data));
        let packed = instruction.pack();
        assert_eq!(packed, expect);
        let unpacked = TokenGroupInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, instruction);
    }

    #[test]
    fn initialize_group_pack() {
        let data = InitializeGroup {
            update_authority: OptionalNonZeroPubkey::default(),
            max_size: 100.into(),
        };
        let instruction = TokenGroupInstruction::InitializeGroup(data);
        let preimage = hash::hashv(&[format!("{NAMESPACE}:initialize_token_group").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<InitializeGroup>(instruction, discriminator, data);
    }

    #[test]
    fn update_group_max_size_pack() {
        let data = UpdateGroupMaxSize {
            max_size: 200.into(),
        };
        let instruction = TokenGroupInstruction::UpdateGroupMaxSize(data);
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_group_max_size").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<UpdateGroupMaxSize>(instruction, discriminator, data);
    }

    #[test]
    fn update_authority_pack() {
        let data = UpdateGroupAuthority {
            new_authority: OptionalNonZeroPubkey::default(),
        };
        let instruction = TokenGroupInstruction::UpdateGroupAuthority(data);
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_authority").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<UpdateGroupAuthority>(instruction, discriminator, data);
    }

    #[test]
    fn initialize_member_pack() {
        let data = InitializeMember {};
        let instruction = TokenGroupInstruction::InitializeMember(data);
        let preimage = hash::hashv(&[format!("{NAMESPACE}:initialize_member").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<InitializeMember>(instruction, discriminator, data);
    }
}
