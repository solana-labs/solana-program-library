use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        msg,
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack, Sealed},
        pubkey::Pubkey,
    },
};

/// The data for a Name Registry account is always prefixed a `NameRecordHeader` structure.
///
/// The layout of the remaining bytes in the account data are determined by the record `class`
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct NameRecordHeader {
    // Names are hierarchical.  `parent_name` contains the account address of the parent
    // name, or `Pubkey::default()` if no parent exists.
    pub parent_name: Pubkey,

    // The owner of this name
    pub owner: Pubkey,

    // The class of data this account represents (DNS record, twitter handle, SPL Token name/symbol, etc)
    //
    // If `Pubkey::default()` the data is unspecified.
    pub class: Pubkey,
}

impl Sealed for NameRecordHeader {}

impl Pack for NameRecordHeader {
    const LEN: usize = 96;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut p = src;
        NameRecordHeader::deserialize(&mut p).map_err(|_| {
            msg!("Failed to deserialize name record");
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for NameRecordHeader {
    fn is_initialized(&self) -> bool {
        self.owner == Pubkey::default()
    }
}

pub fn write_data(account: &AccountInfo, input: &[u8], offset: usize) {
    let mut account_data = account.data.borrow_mut();
    account_data[offset..offset + input.len()].copy_from_slice(input);
}

////////////////////////////////////////////////////////////

pub const HASH_PREFIX: &str = "SPL Name Service";

////////////////////////////////////////////////////////////

pub fn get_seeds_and_key(
    program_id: &Pubkey,
    hashed_name: Vec<u8>, // Hashing is done off-chain
    name_class_opt: Option<&Pubkey>,
    parent_name_address_opt: Option<&Pubkey>,
) -> (Pubkey, Vec<u8>) {
    // let hashed_name: Vec<u8> = hashv(&[(HASH_PREFIX.to_owned() + name).as_bytes()]).0.to_vec();
    let mut seeds_vec: Vec<u8> = hashed_name;

    let name_class = name_class_opt.cloned().unwrap_or_default();

    for b in name_class.to_bytes().to_vec() {
        seeds_vec.push(b);
    }

    let parent_name_address = parent_name_address_opt.cloned().unwrap_or_default();

    for b in parent_name_address.to_bytes().to_vec() {
        seeds_vec.push(b);
    }

    let (name_account_key, bump) =
        Pubkey::find_program_address(&seeds_vec.chunks(32).collect::<Vec<&[u8]>>(), program_id);
    seeds_vec.push(bump);

    (name_account_key, seeds_vec)
}
