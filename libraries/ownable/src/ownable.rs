//! Ownable type

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ownable {
    pub owner: COption<Pubkey>,
}
impl Sealed for Ownable {}
impl IsInitialized for Ownable {
    fn is_initialized(&self) -> bool { true }
}
impl Pack for Ownable {
    const LEN: usize = 4 + 32;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        return Self::unpack_starting_at(src, 0);
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        Self::pack_starting_at(&self, dst, 0);
    }
}
impl Ownable {
    pub fn unpack_starting_at(src: &[u8], offset: usize) -> Result<Self, ProgramError> {
        let src = array_ref![src, offset, Ownable::LEN];
        Ok(Ownable {owner: unpack_coption_key(src)? })
        //Ok(Ownable {owner: Pubkey::new_from_array(*src)})
    }

    pub fn pack_starting_at(&self, dst: &mut [u8], offset: usize) {
        let dst = array_mut_ref![dst, offset, Ownable::LEN];
        let &Ownable { ref owner } = self;
        //owner_dst.copy_from_slice(owner.as_ref());
        pack_coption_key(owner, dst);
    }
}

// Helpers
fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    let (tag, body) = mut_array_refs![dst, 4, 32];
    match src {
        COption::Some(key) => {
            *tag = [1, 0, 0, 0];
            body.copy_from_slice(key.as_ref());
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}
fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

#[cfg(test)]
mod tests {
    use super::Ownable;
    use solana_program::{
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
    };

    #[test]
    fn test_packing() {
        let owner = Pubkey::new(&[3; 32]);
        let ownable = Ownable{ owner: COption::Some(owner) };

        let offset: usize = 5;
        let packed = &mut vec![0; Ownable::get_packed_len() + offset];
        packed[0] = 5;
        packed[4] = 5;
        Ownable::pack_starting_at(&ownable, packed, offset);

        let expect = vec![
            5, 0, 0, 0, 5,
            1, 0, 0, 0,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        ];
        assert_eq!(*packed, expect);

        let unpacked = Ownable::unpack_starting_at(packed, offset).unwrap();
        assert_eq!(unpacked.owner, COption::Some(owner));
    }
}
