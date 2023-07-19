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

/// Instruction data for creating a new `Original` print
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_editions_interface:create_original")]
pub struct CreateOriginal {
    /// Update authority for the original print
    pub update_authority: OptionalNonZeroPubkey,
    /// The maximum supply of copies of this print
    pub max_supply: Option<u64>,
}

/// Update the max supply of an `Original` print
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_editions_interface:update_original_max_supply")]
pub struct UpdateOriginalMaxSupply {
    /// New max supply for the original print
    pub max_supply: Option<u64>,
}

/// Update authority instruction data
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_editions_interface:update_original_authority")]
pub struct UpdateOriginalAuthority {
    /// New authority for the original print, or unset if `None`
    pub new_authority: OptionalNonZeroPubkey,
}

/// Instruction data for creating a new `Reprint` of an `Original` print
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_editions_interface:create_reprint")]
pub struct CreateReprint {
    /// The pubkey of the `Original` print
    pub original: Pubkey,
}

/// Instruction data for Emit
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_token_editions_interface:emitter")]
pub struct Emit {
    /// Which type of print to emit
    pub print_type: PrintType,
    /// Start of range of data to emit
    pub start: Option<u64>,
    /// End of range of data to emit
    pub end: Option<u64>,
}

/// The type of print
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum PrintType {
    /// Original print
    Original,
    /// Reprint
    Reprint,
}

/// All instructions that must be implemented in the token-editions interface
#[derive(Clone, Debug, PartialEq)]
pub enum TokenEditionsInstruction {
    /// Create a new `Original` print
    ///
    /// Assumes one has already created a mint and a metadata account for the
    /// print.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Original
    ///   1. `[]` Update authority
    ///   2. `[]` Metadata
    ///   3. `[]` Mint
    ///   4. `[s]` Mint authority
    ///
    /// Data: `CreateOriginal`: max_supply: `Option<u64>`
    CreateOriginal(CreateOriginal),

    /// Update the max supply of an `Original` print
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Original
    ///   1. `[s]` Update authority
    ///
    /// Data: `UpdateOriginalMaxSupply`: max_supply: `Option<u64>`
    UpdateOriginalMaxSupply(UpdateOriginalMaxSupply),

    /// Update the authority of an `Original` print
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Original
    ///   1. `[s]` Current update authority
    ///
    /// Data: the new authority. Can be unset using a `None` value
    UpdateOriginalAuthority(UpdateOriginalAuthority),

    /// Create a new `Reprint` of an `Original` print
    ///
    /// Assumes the `Original` print has already been created.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Reprint
    ///   1. `[w]` Reprint Metadata
    ///   2. `[]` Reprint Mint
    ///   3. `[]` Original
    ///   4. `[]` Update authority
    ///   5. `[]` Original Metadata
    ///   6. `[]` Original Mint
    ///   7. `[s]` Mint authority
    ///
    /// Data: `CreateReprint`: original: `Pubkey`
    CreateReprint(CreateReprint),

    /// Emits the print edition as return data
    ///
    /// The format of the data emitted follows either the `Original` or
    /// `Reprint` struct,  but it's possible that the account data is stored in
    /// another format by the program.
    ///
    /// With this instruction, a program that implements the token-editions
    /// interface can return `Original` or `Reprint` without adhering to the
    /// specific byte layout of the structs in any accounts.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` Original _or_ Reprint account
    Emit(Emit),
}
impl TokenEditionsInstruction {
    /// Unpacks a byte buffer into a
    /// [TokenEditionsInstruction](enum.TokenEditionsInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(ArrayDiscriminator::LENGTH);
        Ok(match discriminator {
            CreateOriginal::SPL_DISCRIMINATOR_SLICE => {
                let data = CreateOriginal::try_from_slice(rest)?;
                Self::CreateOriginal(data)
            }
            UpdateOriginalMaxSupply::SPL_DISCRIMINATOR_SLICE => {
                let data = UpdateOriginalMaxSupply::try_from_slice(rest)?;
                Self::UpdateOriginalMaxSupply(data)
            }
            UpdateOriginalAuthority::SPL_DISCRIMINATOR_SLICE => {
                let data = UpdateOriginalAuthority::try_from_slice(rest)?;
                Self::UpdateOriginalAuthority(data)
            }
            CreateReprint::SPL_DISCRIMINATOR_SLICE => {
                let data = CreateReprint::try_from_slice(rest)?;
                Self::CreateReprint(data)
            }
            Emit::SPL_DISCRIMINATOR_SLICE => {
                let data = Emit::try_from_slice(rest)?;
                Self::Emit(data)
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [TokenEditionsInstruction](enum.TokenEditionsInstruction.html)
    /// into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::CreateOriginal(data) => {
                buf.extend_from_slice(CreateOriginal::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::UpdateOriginalMaxSupply(data) => {
                buf.extend_from_slice(UpdateOriginalMaxSupply::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::UpdateOriginalAuthority(data) => {
                buf.extend_from_slice(UpdateOriginalAuthority::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::CreateReprint(data) => {
                buf.extend_from_slice(CreateReprint::SPL_DISCRIMINATOR_SLICE);
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

/// Creates a `CreateOriginal` instruction
pub fn create_original(
    program_id: &Pubkey,
    original: &Pubkey,
    update_authority: Option<Pubkey>,
    metadata: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    max_supply: Option<u64>,
) -> Instruction {
    let (update_authority, update_authority_pubkey) = {
        (
            OptionalNonZeroPubkey::try_from(update_authority)
                .expect("Failed to deserialize pubkey for update authority"),
            match update_authority {
                Some(pubkey) => pubkey,
                None => *program_id,
            },
        )
    };
    let data = TokenEditionsInstruction::CreateOriginal(CreateOriginal {
        update_authority,
        max_supply,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*original, false),
            AccountMeta::new_readonly(update_authority_pubkey, false),
            AccountMeta::new_readonly(*metadata, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*mint_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates a `UpdateOriginalMaxSupply` instruction
pub fn update_original_max_supply(
    program_id: &Pubkey,
    original: &Pubkey,
    update_authority: &Pubkey,
    max_supply: Option<u64>,
) -> Instruction {
    let data =
        TokenEditionsInstruction::UpdateOriginalMaxSupply(UpdateOriginalMaxSupply { max_supply });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*original, false),
            AccountMeta::new_readonly(*update_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates a `UpdateOriginalAuthority` instruction
pub fn update_original_authority(
    program_id: &Pubkey,
    original: &Pubkey,
    current_authority: &Pubkey,
    new_authority: Option<Pubkey>,
) -> Instruction {
    let new_authority = OptionalNonZeroPubkey::try_from(new_authority)
        .expect("Failed to deserialize pubkey for update authority");
    let data = TokenEditionsInstruction::UpdateOriginalAuthority(UpdateOriginalAuthority {
        new_authority,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*original, false),
            AccountMeta::new_readonly(*current_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates a `CreateReprint` instruction
#[allow(clippy::too_many_arguments)]
pub fn create_reprint(
    program_id: &Pubkey,
    reprint: &Pubkey,
    reprint_metadata: &Pubkey,
    reprint_mint: &Pubkey,
    original: &Pubkey,
    update_authority: &Pubkey,
    original_metadata: &Pubkey,
    original_mint: &Pubkey,
    mint_authority: &Pubkey,
) -> Instruction {
    let data = TokenEditionsInstruction::CreateReprint(CreateReprint {
        original: *original,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*reprint, false),
            AccountMeta::new(*reprint_metadata, false),
            AccountMeta::new_readonly(*reprint_mint, false),
            AccountMeta::new(*original, false),
            AccountMeta::new_readonly(*update_authority, true),
            AccountMeta::new_readonly(*original_metadata, false),
            AccountMeta::new_readonly(*original_mint, false),
            AccountMeta::new_readonly(*mint_authority, true),
        ],
        data: data.pack(),
    }
}

/// Creates an `Emit` instruction
pub fn emit(
    program_id: &Pubkey,
    print: &Pubkey,
    print_type: PrintType,
    start: Option<u64>,
    end: Option<u64>,
) -> Instruction {
    let data = TokenEditionsInstruction::Emit(Emit {
        print_type,
        start,
        end,
    });
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new_readonly(*print, false)],
        data: data.pack(),
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    fn check_pack_unpack<T: BorshSerialize>(
        instruction: TokenEditionsInstruction,
        discriminator: &[u8],
        data: T,
    ) {
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.append(&mut data.try_to_vec().unwrap());
        let packed = instruction.pack();
        assert_eq!(packed, expect);
        let unpacked = TokenEditionsInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, instruction);
    }

    #[test]
    fn create_original_pack() {
        let data = CreateOriginal {
            update_authority: OptionalNonZeroPubkey::default(),
            max_supply: Some(100),
        };
        let check = TokenEditionsInstruction::CreateOriginal(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:create_original").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn update_original_max_supply_pack() {
        let data = UpdateOriginalMaxSupply {
            max_supply: Some(200),
        };
        let check = TokenEditionsInstruction::UpdateOriginalMaxSupply(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_original_max_supply").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn update_authority_pack() {
        let data = UpdateOriginalAuthority {
            new_authority: OptionalNonZeroPubkey::default(),
        };
        let check = TokenEditionsInstruction::UpdateOriginalAuthority(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:update_original_authority").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn create_reprint_pack() {
        let data = CreateReprint {
            original: Pubkey::new_unique(),
        };
        let check = TokenEditionsInstruction::CreateReprint(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:create_reprint").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }

    #[test]
    fn emit_pack() {
        let data = Emit {
            print_type: PrintType::Original,
            start: None,
            end: Some(10),
        };
        let check = TokenEditionsInstruction::Emit(data.clone());
        let preimage = hash::hashv(&[format!("{NAMESPACE}:emitter").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        check_pack_unpack(check, discriminator, data);
    }
}
