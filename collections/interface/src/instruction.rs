//! Instruction types

use {
    crate::state::OptionalNonZeroPubkey,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
};

/// Instruction data for creating a new `Collection`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_collections_interface:create_collection")]
pub struct CreateCollection {
    /// Update authority for the collection
    pub update_authority: OptionalNonZeroPubkey,
    /// The maximum number of collection members
    pub max_size: Option<u64>,
}

/// Update the max size of a `Collection`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_collections_interface:update_collection_max_size")]
pub struct UpdateCollectionMaxSize {
    /// New max size for the collection
    pub max_size: Option<u64>,
}

/// Update authority instruction data
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_collections_interface:update_collection_authority")]
pub struct UpdateCollectionAuthority {
    /// New authority for the collection, or unset if `None`
    pub new_authority: OptionalNonZeroPubkey,
}

/// Instruction data for creating a new `Member` of a `Collection`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_collections_interface:create_member")]
pub struct CreateMember {
    /// The pubkey of the `Collection`
    pub collection: Pubkey,
}

/// Instruction data for Emit
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_collections_interface:emitter")]
pub struct Emit {
    /// Which type of item to emit
    pub item_type: ItemType,
    /// Start of range of data to emit
    pub start: Option<u64>,
    /// End of range of data to emit
    pub end: Option<u64>,
}

/// The type of item
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum ItemType {
    /// Collection
    Collection,
    /// Member
    Member,
}

/// All instructions that must be implemented in the token-collections interface
#[derive(Clone, Debug, PartialEq)]
pub enum TokenCollectionsInstruction {
    /// Create a new `Collection`
    ///
    /// Assumes one has already created a mint for the
    /// collection.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Collection
    ///   1. `[]`   Mint
    ///   2. `[s]`  Mint authority
    ///
    /// Data: `CreateCollection`: max_size: `Option<u64>`
    CreateCollection(CreateCollection),

    /// Update the max size of a `Collection`
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Collection
    ///   1. `[s]`  Update authority
    ///
    /// Data: `UpdateCollectionMaxSize`: max_size: `Option<u64>`
    UpdateCollectionMaxSize(UpdateCollectionMaxSize),

    /// Update the authority of a `Collection`
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Collection
    ///   1. `[s]`  Current update authority
    ///
    /// Data: the new authority. Can be unset using a `None` value
    UpdateCollectionAuthority(UpdateCollectionAuthority),

    /// Create a new `Member` of a `Collection`
    ///
    /// Assumes the `Collection` has already been created,
    /// as well as the mint for the member.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`  Member
    ///   1. `[]`   Member Mint
    ///   2. `[s]`  Member Mint authority
    ///   3. `[w]`  Collection
    ///   4. `[]`   Collection Mint
    ///   5. `[s]`  Collection Mint authority
    ///
    /// Data: `CreateMember`: collection: `Pubkey`
    CreateMember(CreateMember),

    /// Emits the collection or member as return data
    ///
    /// The format of the data emitted follows either the `Collection` or
    /// `Member` struct,  but it's possible that the account data is stored in
    /// another format by the program.
    ///
    /// With this instruction, a program that implements the token-collections
    /// interface can return `Collection` or `Member` without adhering to the
    /// specific byte layout of the structs in any accounts.
    ///
    /// The dictation of which data to emit is determined by the `ItemType`
    /// enum argument to the instruction data.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]`   Collection _or_ Member account
    Emit(Emit),
}
impl TokenCollectionsInstruction {
    /// Unpacks a byte buffer into a
    /// [TokenCollectionsInstruction](enum.TokenCollectionsInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(ArrayDiscriminator::LENGTH);
        Ok(match discriminator {
            CreateCollection::SPL_DISCRIMINATOR_SLICE => {
                let data = CreateCollection::try_from_slice(rest)?;
                Self::CreateCollection(data)
            }
            UpdateCollectionMaxSize::SPL_DISCRIMINATOR_SLICE => {
                let data = UpdateCollectionMaxSize::try_from_slice(rest)?;
                Self::UpdateCollectionMaxSize(data)
            }
            UpdateCollectionAuthority::SPL_DISCRIMINATOR_SLICE => {
                let data = UpdateCollectionAuthority::try_from_slice(rest)?;
                Self::UpdateCollectionAuthority(data)
            }
            CreateMember::SPL_DISCRIMINATOR_SLICE => {
                let data = CreateMember::try_from_slice(rest)?;
                Self::CreateMember(data)
            }
            Emit::SPL_DISCRIMINATOR_SLICE => {
                let data = Emit::try_from_slice(rest)?;
                Self::Emit(data)
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [TokenCollectionsInstruction](enum.TokenCollectionsInstruction.
    /// html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::CreateCollection(data) => {
                buf.extend_from_slice(CreateCollection::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::UpdateCollectionMaxSize(data) => {
                buf.extend_from_slice(UpdateCollectionMaxSize::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::UpdateCollectionAuthority(data) => {
                buf.extend_from_slice(UpdateCollectionAuthority::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::CreateMember(data) => {
                buf.extend_from_slice(CreateMember::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::Emit(data) => {
                buf.extend_from_slice(Emit::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
        };
        buf
    }
}

/// Creates a `CreateCollection` instruction
pub fn create_collection(
    program_id: &Pubkey,
    collection: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    update_authority: Option<Pubkey>,
    max_size: Option<u64>,
) -> Instruction {
    let update_authority = OptionalNonZeroPubkey::try_from(update_authority)
        .expect("Failed to deserialize `Option<Pubkey>`");
    let data = TokenCollectionsInstruction::CreateCollection(CreateCollection {
        update_authority,
        max_size,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*mint_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates a `UpdateCollectionMaxSize` instruction
pub fn update_collection_max_size(
    program_id: &Pubkey,
    collection: &Pubkey,
    update_authority: &Pubkey,
    max_size: Option<u64>,
) -> Instruction {
    let data =
        TokenCollectionsInstruction::UpdateCollectionMaxSize(UpdateCollectionMaxSize { max_size });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(*update_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates a `UpdateCollectionAuthority` instruction
pub fn update_collection_authority(
    program_id: &Pubkey,
    collection: &Pubkey,
    current_authority: &Pubkey,
    new_authority: Option<Pubkey>,
) -> Instruction {
    let new_authority = OptionalNonZeroPubkey::try_from(new_authority)
        .expect("Failed to deserialize `Option<Pubkey>`");
    let data = TokenCollectionsInstruction::UpdateCollectionAuthority(UpdateCollectionAuthority {
        new_authority,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(*current_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates a `CreateMember` instruction
#[allow(clippy::too_many_arguments)]
pub fn create_member(
    program_id: &Pubkey,
    member: &Pubkey,
    member_mint: &Pubkey,
    member_mint_authority: &Pubkey,
    collection: &Pubkey,
    collection_mint: &Pubkey,
    collection_mint_authority: &Pubkey,
) -> Instruction {
    let data = TokenCollectionsInstruction::CreateMember(CreateMember {
        collection: *collection,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*member, false),
            AccountMeta::new_readonly(*member_mint, false),
            AccountMeta::new_readonly(*member_mint_authority, true),
            AccountMeta::new(*collection, false),
            AccountMeta::new_readonly(*collection_mint, false),
            AccountMeta::new_readonly(*collection_mint_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates an `Emit` instruction
pub fn emit(
    program_id: &Pubkey,
    item: &Pubkey,
    item_type: ItemType,
    start: Option<u64>,
    end: Option<u64>,
) -> Instruction {
    let data = TokenCollectionsInstruction::Emit(Emit {
        item_type,
        start,
        end,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new_readonly(*item, false)],
        data: data.pack(),
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    fn check_pack_unpack<T: BorshSerialize>(
        instruction: TokenCollectionsInstruction,
        discriminator: &[u8],
        data: T,
    ) {
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.append(&mut data.try_to_vec().unwrap());
        let packed = instruction.pack();
        assert_eq!(packed, expect);
        let unpacked = TokenCollectionsInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, instruction);
    }

    #[test]
    fn create_collection_pack() {
        let data = CreateCollection {
            update_authority: OptionalNonZeroPubkey::default(),
            max_size: Some(100),
        };
        let check = TokenCollectionsInstruction::CreateCollection(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:create_collection").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn update_collection_max_size_pack() {
        let data = UpdateCollectionMaxSize {
            max_size: Some(200),
        };
        let check = TokenCollectionsInstruction::UpdateCollectionMaxSize(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_collection_max_size").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn update_authority_pack() {
        let data = UpdateCollectionAuthority {
            new_authority: OptionalNonZeroPubkey::default(),
        };
        let check = TokenCollectionsInstruction::UpdateCollectionAuthority(data.clone());
        let preimage =
            hash::hashv(&[format!("{NAMESPACE}:update_collection_authority").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn create_member_pack() {
        let data = CreateMember {
            collection: Pubkey::new_unique(),
        };
        let check = TokenCollectionsInstruction::CreateMember(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:create_member").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn emit_pack() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:emitter").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];

        let collection_data = Emit {
            item_type: ItemType::Collection,
            start: None,
            end: Some(10),
        };
        let collection_check = TokenCollectionsInstruction::Emit(collection_data.clone());
        check_pack_unpack(collection_check, discriminator, collection_data);

        let member_data = Emit {
            item_type: ItemType::Member,
            start: None,
            end: Some(7),
        };
        let member_check = TokenCollectionsInstruction::Emit(member_data.clone());
        check_pack_unpack(member_check, discriminator, member_data);
    }
}
