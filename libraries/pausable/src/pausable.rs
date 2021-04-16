//! Pausable type

use spl_ownable::ownable::Ownable;
use arrayref::{array_mut_ref, array_ref, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pausable {
    pub ownable: Ownable,
    pub paused: bool,
}
impl Sealed for Pausable {}
impl IsInitialized for Pausable {
    fn is_initialized(&self) -> bool { true }
}
impl Pack for Pausable {
    const LEN: usize = Ownable::LEN + 1;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        return Self::unpack_starting_at(src, 0);
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        Self::pack_starting_at(&self, dst, 0);
    }
}
impl Pausable {
    pub fn unpack_starting_at(src: &[u8], offset: usize) -> Result<Self, ProgramError> {
        let ownable = Ownable::unpack_starting_at(src, offset)?;
        let paused = array_ref![src, offset + Ownable::get_packed_len(), 1];
        let paused = match paused {
            [0] => false,
            [1] => true,
            _ => {
                msg!("Invalid paused boolean value {:?}", paused);
                return Err(ProgramError::InvalidAccountData)
            }
        }; 
        Ok(Pausable {ownable, paused})
    }

    pub fn pack_starting_at(&self, dst: &mut [u8], offset: usize) {
        let dst = array_mut_ref![dst, offset, Pausable::LEN];
        let (ownable_dst, paused_dst) = mut_array_refs![dst, Ownable::LEN, 1];
        let &Pausable { ownable, paused } = self;
        Ownable::pack_into_slice(&ownable, ownable_dst);
        paused_dst[0] = paused as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::Pausable;
    use spl_ownable::ownable::Ownable;
    use solana_program::{
        program_error::ProgramError,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
    };

    #[test]
    fn test_packing() {
        let owner = Pubkey::new(&[3; 32]);
        let ownable = Ownable{ owner: COption::Some(owner) };
        let pausable = Pausable { ownable, paused: true };

        let offset: usize = 5;
        let mut packed = vec![0; Pausable::get_packed_len() + offset];
        packed[4] = 4;
        Pausable::pack_starting_at(&pausable, &mut packed, offset);

        let expect = vec![
            0, 0, 0, 0, 4,
            1, 0, 0, 0,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            1,
        ];
        assert_eq!(packed, expect);

        let unpacked = Pausable::unpack_starting_at(&expect, offset).unwrap();
        assert!(unpacked.paused);

        let mut other = vec![0; Pausable::get_packed_len()];
        let unpacked = Pausable::unpack(&other).unwrap();
        assert!(!unpacked.paused);

        other[Pausable::get_packed_len() - 1] = 1;
        let unpacked = Pausable::unpack(&other).unwrap();
        assert!(unpacked.paused);

        let other = vec![2; Pausable::get_packed_len()];
        assert_eq!(Pausable::unpack(&other), Err(ProgramError::InvalidAccountData));
    }
}
