//! Common routines for packing/unpacking

use {
    crate::string::ArrayString64,
    arrayref::{array_refs, mut_array_refs},
    serde::{
        de::{Error, Visitor},
        Deserialize, Deserializer, Serializer,
    },
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    std::{fmt, str::FromStr},
};

/// Checks if the slice has at least min_len size
pub fn check_data_len(data: &[u8], min_len: usize) -> Result<(), ProgramError> {
    if data.len() < min_len {
        Err(ProgramError::AccountDataTooSmall)
    } else {
        Ok(())
    }
}

/// Converts bool to a byte
pub fn pack_bool(input: bool, output: &mut [u8; 1]) {
    output[0] = input as u8;
}

/// Converts a raw byte to a bool
pub fn unpack_bool(input: &[u8; 1]) -> Result<bool, ProgramError> {
    let result = match input {
        [0] => false,
        [1] => true,
        _ => return Err(ProgramError::InvalidAccountData),
    };
    Ok(result)
}

pub fn pack_option_key(input: &Option<Pubkey>, output: &mut [u8; 33]) {
    let (tag, data) = mut_array_refs![output, 1, 32];
    match input {
        Option::Some(key) => {
            tag[0] = 1;
            data.copy_from_slice(key.as_ref());
        }
        Option::None => {
            tag[0] = 0;
        }
    }
}

pub fn unpack_option_key(input: &[u8; 33]) -> Result<Option<Pubkey>, ProgramError> {
    let (tag, data) = array_refs![input, 1, 32];
    match *tag {
        [0] => Ok(Option::None),
        [1] => Ok(Option::Some(Pubkey::new_from_array(*data))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

pub fn pack_option_u32(input: Option<u32>, output: &mut [u8; 5]) {
    let (tag, data) = mut_array_refs![output, 1, 4];
    match input {
        Option::Some(val) => {
            tag[0] = 1;
            *data = val.to_le_bytes();
        }
        Option::None => {
            tag[0] = 0;
        }
    }
}

pub fn unpack_option_u32(input: &[u8; 5]) -> Result<Option<u32>, ProgramError> {
    let (tag, data) = array_refs![input, 1, 4];
    match *tag {
        [0] => Ok(Option::None),
        [1] => Ok(Option::Some(u32::from_le_bytes(*data))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

pub fn pack_array_string64(input: &ArrayString64, output: &mut [u8; 64]) {
    for (dst, src) in output.iter_mut().zip(input.as_bytes()) {
        *dst = *src
    }
}

pub fn unpack_array_string64(input: &[u8; 64]) -> Result<ArrayString64, ProgramError> {
    if let Some(i) = input.iter().position(|x| *x == 0) {
        ArrayString64::try_from_utf8(&input[0..i]).or(Err(ProgramError::InvalidAccountData))
    } else {
        ArrayString64::try_from_utf8(input).or(Err(ProgramError::InvalidAccountData))
    }
}

/// Custom Pubkey deserializer to use with Serde
pub fn pubkey_deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).unwrap();
    Pubkey::from_str(s.as_str()).map_err(D::Error::custom)
}

/// Custom Pubkey serializer to use with Serde
pub fn pubkey_serialize<S>(x: &Pubkey, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(x.to_string().as_str())
}

/// Custom Option<Pubkey> deserializer to use with Serde
pub fn optional_pubkey_deserialize<'de, D>(deserializer: D) -> Result<Option<Pubkey>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).unwrap();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            Pubkey::from_str(s.as_str()).map_err(D::Error::custom)?,
        ))
    }
}

/// Custom Option<Pubkey> serializer to use with Serde
pub fn optional_pubkey_serialize<S>(x: &Option<Pubkey>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(key) = x {
        s.serialize_str(key.to_string().as_str())
    } else {
        s.serialize_str("")
    }
}

/// Custom ArrayString64 serializer to use with Serde
pub fn as64_serialize<S>(x: &ArrayString64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(x.as_str())
}

/// Custom ArrayString64 deserializer to use with Serde
pub fn as64_deserialize<'de, D>(deserializer: D) -> Result<ArrayString64, D::Error>
where
    D: Deserializer<'de>,
{
    struct ArrayStringVisitor;

    impl<'de> Visitor<'de> for ArrayStringVisitor {
        type Value = ArrayString64;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            ArrayString64::try_from_str(v).map_err(E::custom)
        }
    }

    deserializer.deserialize_any(ArrayStringVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as64_serialization() {
        let as1 = ArrayString64::from_utf8("test").unwrap();
        let mut output: [u8; 64] = [0; 64];
        pack_array_string64(&as1, &mut output);
        let as2 = unpack_array_string64(&output).unwrap();
        assert_eq!(as1, as2);
    }

    #[test]
    fn test_as64_serialization_utf8() {
        let as1 = ArrayString64::from_utf8("тест").unwrap();
        let mut output: [u8; 64] = [0; 64];
        pack_array_string64(&as1, &mut output);
        let as2 = unpack_array_string64(&output).unwrap();
        assert_eq!(as1, as2);
    }
}
