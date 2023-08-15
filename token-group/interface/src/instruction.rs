//! Instruction types

use {
    crate::state::SplTokenGroup,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_interface_base::state::OptionalNonZeroPubkey,
};

/// Instruction data for initializing a new `Group`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:initialize_group")]
pub struct InitializeGroup<G>
where
    G: SplTokenGroup,
{
    /// Update authority for the group
    pub update_authority: OptionalNonZeroPubkey,
    /// The maximum number of group members
    pub max_size: Option<u64>,
    /// Additional state
    pub meta: Option<G>,
}

/// Instruction data for updating the max size of a `Group`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:update_group_max_size")]
pub struct UpdateGroupMaxSize {
    /// New max size for the group
    pub max_size: Option<u64>,
}

/// Instruction data for updating the authority of a `Group`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:update_group_authority")]
pub struct UpdateGroupAuthority {
    /// New authority for the group, or unset if `None`
    pub new_authority: OptionalNonZeroPubkey,
}

/// Instruction data for initializing a new `Member` of a `Group`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_group_interface:initialize_member")]
pub struct InitializeMember {
    /// The pubkey of the `Group`
    pub group: Pubkey,
    /// The member number
    pub member_number: u64,
}

/// All instructions that must be implemented in the SPL Token Group Interface
///
/// Note: Any instruction can be extended using additional required accounts by
/// using the `InitializeExtraAccountMetaList` instruction to write
/// configurations for extra required accounts into validation data
/// corresponding to an instruction's unique discriminator.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenGroupInterfaceInstruction<G>
where
    G: SplTokenGroup,
{
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
    InitializeGroup(InitializeGroup<G>),

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
    ///   1. `[]`   Member Mint
    ///   2. `[s]`  Member Mint authority
    ///   3. `[w]`  Group
    ///   4. `[]`   Group Mint
    ///   5. `[s]`  Group Mint authority
    InitializeMember(InitializeMember),
}
impl<G> TokenGroupInterfaceInstruction<G>
where
    G: SplTokenGroup,
{
    /// Unpacks a byte buffer into a `TokenGroupInterfaceInstruction`
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Should have at least _two_ leading discriminators
        if input.len() < ArrayDiscriminator::LENGTH * 2 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = {
            let (discriminators, rest) = input.split_at(ArrayDiscriminator::LENGTH * 2);
            let (generic_discriminator, instruction_discriminator) =
                discriminators.split_at(ArrayDiscriminator::LENGTH);
            if !generic_discriminator.eq(G::SPL_DISCRIMINATOR_SLICE) {
                return Err(ProgramError::InvalidInstructionData);
            }
            (instruction_discriminator, rest)
        };
        Ok(match discriminator {
            InitializeGroup::<G>::SPL_DISCRIMINATOR_SLICE => {
                let data = InitializeGroup::try_from_slice(rest)?;
                Self::InitializeGroup(data)
            }
            UpdateGroupMaxSize::SPL_DISCRIMINATOR_SLICE => {
                let data = UpdateGroupMaxSize::try_from_slice(rest)?;
                Self::UpdateGroupMaxSize(data)
            }
            UpdateGroupAuthority::SPL_DISCRIMINATOR_SLICE => {
                let data = UpdateGroupAuthority::try_from_slice(rest)?;
                Self::UpdateGroupAuthority(data)
            }
            InitializeMember::SPL_DISCRIMINATOR_SLICE => {
                let data = InitializeMember::try_from_slice(rest)?;
                Self::InitializeMember(data)
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a `TokenGroupInterfaceInstruction` into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        // The first discriminator is the generic discriminator
        buf.extend_from_slice(G::SPL_DISCRIMINATOR_SLICE);
        // The second discriminator is the instruction-specific discriminator
        match self {
            Self::InitializeGroup(data) => {
                buf.extend_from_slice(InitializeGroup::<G>::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::UpdateGroupMaxSize(data) => {
                buf.extend_from_slice(UpdateGroupMaxSize::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::UpdateGroupAuthority(data) => {
                buf.extend_from_slice(UpdateGroupAuthority::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::InitializeMember(data) => {
                buf.extend_from_slice(InitializeMember::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
        };
        buf
    }

    /// Peeks the instruction data to determine its generic implementation
    pub fn peek(input: &[u8]) -> bool {
        input[..8].eq(G::SPL_DISCRIMINATOR_SLICE)
    }
}

/// Creates a `InitializeGroup` instruction
pub fn initialize_group<G>(
    program_id: &Pubkey,
    group: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    update_authority: Option<Pubkey>,
    max_size: Option<u64>,
    meta: &Option<G>,
) -> Instruction
where
    G: SplTokenGroup,
{
    let update_authority = OptionalNonZeroPubkey::try_from(update_authority)
        .expect("Failed to deserialize `Option<Pubkey>`");
    let data = TokenGroupInterfaceInstruction::<G>::InitializeGroup(InitializeGroup {
        update_authority,
        max_size,
        meta: meta.clone(),
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
pub fn update_group_max_size<G>(
    program_id: &Pubkey,
    group: &Pubkey,
    update_authority: &Pubkey,
    max_size: Option<u64>,
) -> Instruction
where
    G: SplTokenGroup,
{
    let data =
        TokenGroupInterfaceInstruction::<G>::UpdateGroupMaxSize(UpdateGroupMaxSize { max_size })
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
pub fn update_group_authority<G>(
    program_id: &Pubkey,
    group: &Pubkey,
    current_authority: &Pubkey,
    new_authority: Option<Pubkey>,
) -> Instruction
where
    G: SplTokenGroup,
{
    let new_authority = OptionalNonZeroPubkey::try_from(new_authority)
        .expect("Failed to deserialize `Option<Pubkey>`");
    let data = TokenGroupInterfaceInstruction::<G>::UpdateGroupAuthority(UpdateGroupAuthority {
        new_authority,
    })
    .pack();
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
pub fn initialize_member<G>(
    program_id: &Pubkey,
    group: &Pubkey,
    group_mint: &Pubkey,
    group_mint_authority: &Pubkey,
    member: &Pubkey,
    member_mint: &Pubkey,
    member_mint_authority: &Pubkey,
    member_number: u64,
) -> Instruction
where
    G: SplTokenGroup,
{
    let data = TokenGroupInterfaceInstruction::<G>::InitializeMember(InitializeMember {
        group: *group,
        member_number,
    })
    .pack();
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*member, false),
            AccountMeta::new_readonly(*member_mint, false),
            AccountMeta::new_readonly(*member_mint_authority, true),
            AccountMeta::new(*group, false),
            AccountMeta::new_readonly(*group_mint, false),
            AccountMeta::new_readonly(*group_mint_authority, true),
        ],
        data,
    }
}

#[cfg(test)]
mod test {
    use {
        super::*, crate::NAMESPACE, solana_program::hash,
        spl_type_length_value::SplBorshVariableLenPack,
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
    struct MockGroup;
    impl SplTokenGroup for MockGroup {}

    fn instruction_pack_unpack<I, G>(
        instruction: TokenGroupInterfaceInstruction<G>,
        discriminator: &[u8],
        data: I,
    ) where
        I: core::fmt::Debug + PartialEq + BorshDeserialize + BorshSerialize + SplDiscriminate,
        G: core::fmt::Debug + PartialEq + SplTokenGroup,
    {
        let mut expect = vec![];
        expect.extend_from_slice(G::SPL_DISCRIMINATOR_SLICE);
        expect.extend_from_slice(discriminator.as_ref());
        expect.append(&mut data.try_to_vec().unwrap());
        let packed = instruction.pack();
        assert_eq!(packed, expect);
        let unpacked = TokenGroupInterfaceInstruction::<G>::unpack(&expect).unwrap();
        assert_eq!(unpacked, instruction);
    }

    #[test]
    fn initialize_group_pack() {
        let data = InitializeGroup {
            update_authority: OptionalNonZeroPubkey::default(),
            max_size: Some(100),
            meta: None,
        };
        let instruction = TokenGroupInterfaceInstruction::InitializeGroup(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:initialize_group").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<InitializeGroup<MockGroup>, MockGroup>(
            instruction,
            discriminator,
            data,
        );
    }

    #[test]
    fn update_group_max_size_pack() {
        let data = UpdateGroupMaxSize {
            max_size: Some(200),
        };
        let instruction = TokenGroupInterfaceInstruction::UpdateGroupMaxSize(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_group_max_size").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<UpdateGroupMaxSize, MockGroup>(instruction, discriminator, data);
    }

    #[test]
    fn update_authority_pack() {
        let data = UpdateGroupAuthority {
            new_authority: OptionalNonZeroPubkey::default(),
        };
        let instruction = TokenGroupInterfaceInstruction::UpdateGroupAuthority(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_group_authority").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<UpdateGroupAuthority, MockGroup>(
            instruction,
            discriminator,
            data,
        );
    }

    #[test]
    fn initialize_member_pack() {
        let data = InitializeMember {
            group: Pubkey::new_unique(),
            member_number: 100,
        };
        let instruction = TokenGroupInterfaceInstruction::InitializeMember(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:initialize_member").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        instruction_pack_unpack::<InitializeMember, MockGroup>(instruction, discriminator, data);
    }
}
