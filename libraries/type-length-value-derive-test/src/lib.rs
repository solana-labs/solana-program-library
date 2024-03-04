//! Test crate to avoid making `borsh` a direct dependency of
//! `spl-type-length-value`. You can't use a derive macro from within the same
//! crate that the macro is defined, so we need this extra crate for just
//! testing the macro itself.

#[cfg(test)]
pub mod test {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        solana_program::borsh1::{get_instance_packed_len, try_from_slice_unchecked},
        spl_discriminator::SplDiscriminate,
        spl_type_length_value::{variable_len_pack::VariableLenPack, SplBorshVariableLenPack},
    };

    #[derive(
        Clone,
        Debug,
        Default,
        PartialEq,
        BorshDeserialize,
        BorshSerialize,
        SplDiscriminate,
        SplBorshVariableLenPack,
    )]
    #[discriminator_hash_input("vehicle::my_vehicle")]
    pub struct Vehicle {
        vin: [u8; 8],
        plate: [u8; 7],
    }

    #[test]
    fn test_derive() {
        let vehicle = Vehicle {
            vin: [0; 8],
            plate: [0; 7],
        };

        assert_eq!(
            get_instance_packed_len::<Vehicle>(&vehicle).unwrap(),
            vehicle.get_packed_len().unwrap()
        );

        let dst1 = &mut [0u8; 15];
        borsh::to_writer(&mut dst1[..], &vehicle).unwrap();

        let dst2 = &mut [0u8; 15];
        vehicle.pack_into_slice(&mut dst2[..]).unwrap();

        assert_eq!(dst1, dst2,);

        let mut buffer = [0u8; 15];
        buffer.copy_from_slice(&dst1[..]);

        assert_eq!(
            try_from_slice_unchecked::<Vehicle>(&buffer).unwrap(),
            Vehicle::unpack_from_slice(&buffer).unwrap()
        );
    }
}
