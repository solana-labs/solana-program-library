//! serialization module - contains helpers for serde types from other crates, deserialization visitors

use {
    base64::{Engine, prelude::BASE64_STANDARD},
    serde::de::Error
};

/// helper function to convert base64 encoded string into a bytes array
fn base64_to_bytes<const N: usize, E: Error>(v: &str) -> Result<[u8; N], E> {
    let bytes = BASE64_STANDARD.decode(v).map_err(Error::custom)?;

    if bytes.len() != N {
        return Err(Error::custom(format!("Length of base64 decoded bytes is not {}", N)));
    }

    let mut array = [0; N];
    array.copy_from_slice(&bytes[0..N]);
    Ok(array)
}

/// helper function to ser/deser COption wrapped values
pub mod coption_fromstr {
    use {
        serde::{
            de::{Error, Unexpected, Visitor},
            Deserializer, Serializer,
        },
        solana_program::program_option::COption,
        std::{
            fmt::{self, Display},
            marker::PhantomData,
            str::FromStr,
        },
    };

    /// serialize values supporting Display trait wrapped in COption
    pub fn serialize<S, T>(x: &COption<T>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Display,
    {
        match *x {
            COption::Some(ref value) => s.serialize_some(&value.to_string()),
            COption::None => s.serialize_none(),
        }
    }

    struct COptionVisitor<T> {
        s: PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for COptionVisitor<T>
    where
        T: FromStr,
    {
        type Value = COption<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a FromStr type")
        }

        fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            d.deserialize_str(self)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            T::from_str(v)
                .map(|r| COption::Some(r))
                .map_err(|_| E::invalid_value(Unexpected::Str(v), &"value string"))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(COption::None)
        }
    }

    /// deserialize values supporting Display trait wrapped in COption
    pub fn deserialize<'de, D, T>(d: D) -> Result<COption<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
    {
        d.deserialize_option(COptionVisitor {
            s: PhantomData::default(),
        })
    }
}

/// helper to ser/deser AeCiphertext values
pub mod aeciphertext_fromstr {
    use {
        serde::{
            de::{Error, Visitor},
            Deserializer, Serializer,
        },
        solana_zk_token_sdk::zk_token_elgamal::pod::AeCiphertext,
        std::fmt,
    };

    const AE_CIPHERTEXT_LEN: usize = 36;

    /// serialize AeCiphertext values supporting Display trait
    pub fn serialize<S>(x: &AeCiphertext, s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        s.serialize_str(&x.to_string())
    }

    struct AeCiphertextVisitor;

    impl<'de> Visitor<'de> for AeCiphertextVisitor {
        type Value = AeCiphertext;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a FromStr type")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
        {
            let array = super::base64_to_bytes::<AE_CIPHERTEXT_LEN, E>(v)?;
            Ok(AeCiphertext(array))
        }
    }

    /// deserialize AeCiphertext values from str
    pub fn deserialize<'de, D>(d: D) -> Result<AeCiphertext, D::Error>
        where
            D: Deserializer<'de>,
    {
        d.deserialize_str(AeCiphertextVisitor)
    }
}


/// helper to ser/deser pod::ElGamalPubkey values
pub mod elgamalpubkey_fromstr {
    use {
        serde::{
            de::{Error, Visitor},
            Deserializer, Serializer,
        },
        solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalPubkey,
        std::fmt,
    };

    const ELGAMAL_PUBKEY_LEN: usize = 32;

    /// serialize ElGamalPubkey values supporting Display trait
    pub fn serialize<S>(x: &ElGamalPubkey, s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        s.serialize_str(&x.to_string())
    }

    struct ElGamalPubkeyVisitor;

    impl<'de> Visitor<'de> for ElGamalPubkeyVisitor {
        type Value = ElGamalPubkey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a FromStr type")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
        {
            let array = super::base64_to_bytes::<ELGAMAL_PUBKEY_LEN, E>(v)?;
            Ok(ElGamalPubkey(array))
        }
    }

    /// deserialize ElGamalPubkey values from str
    pub fn deserialize<'de, D>(d: D) -> Result<ElGamalPubkey, D::Error>
        where
            D: Deserializer<'de>,
    {
        d.deserialize_str(ElGamalPubkeyVisitor)
    }
}

/// deserialization Visitors for local types
pub mod visitors {
    use {
        crate::{
            pod::OptionalNonZeroElGamalPubkey,
            pod::OptionalNonZeroPubkey,
        },
        serde::{
            de::{Error, Unexpected, Visitor},
        },
        solana_program::{pubkey::Pubkey},
        solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalPubkey,
        std::{convert::TryFrom, fmt, str::FromStr}
    };

    /// Visitor for deserializing OptionalNonZeroPubkey
    pub(crate) struct OptionalNonZeroPubkeyVisitor;

    impl<'de> Visitor<'de> for OptionalNonZeroPubkeyVisitor {
        type Value = OptionalNonZeroPubkey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a Pubkey in base58 or `null`")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
        {
            let pkey = Pubkey::from_str(&v)
                    .map_err(|_| Error::invalid_value(Unexpected::Str(v), &"value string"))?;

            OptionalNonZeroPubkey::try_from(Some(pkey))
                .map_err(|_| Error::custom("Failed to convert from pubkey"))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
        {
            OptionalNonZeroPubkey::try_from(None)
                .map_err(|e| Error::custom(e.to_string()))
        }
    }

    /// Visitor for deserializing OptionalNonZeroElGamalPubkey
    pub(crate) struct OptionalNonZeroElGamalPubkeyVisitor;
    const OPTIONAL_NZ_ELG_LEN: usize = 32;

    impl<'de> Visitor<'de> for OptionalNonZeroElGamalPubkeyVisitor {
        type Value = OptionalNonZeroElGamalPubkey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an ElGamal public key as base64 or `null`")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
        {
            let array = super::base64_to_bytes::<OPTIONAL_NZ_ELG_LEN, E>(v)?;
            let elgamal_pubkey = ElGamalPubkey(array);
            OptionalNonZeroElGamalPubkey::try_from(Some(elgamal_pubkey)).map_err(Error::custom)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
        {
            Ok(OptionalNonZeroElGamalPubkey::default())
        }
    }
}