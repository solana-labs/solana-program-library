//! Serialization module - contains helpers for serde types from other crates,
//! deserialization visitors

/// Helper function to serialize / deserialize `COption` wrapped values
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

    /// Serialize values supporting `Display` trait wrapped in `COption`
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

    /// Deserialize values supporting `Display` trait wrapped in `COption`
    pub fn deserialize<'de, D, T>(d: D) -> Result<COption<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
    {
        d.deserialize_option(COptionVisitor { s: PhantomData })
    }
}

/// Helper to serialize / deserialize `PodAeCiphertext` values
pub mod aeciphertext_fromstr {
    use {
        serde::{
            de::{Error, Visitor},
            Deserializer, Serializer,
        },
        solana_zk_sdk::encryption::pod::auth_encryption::PodAeCiphertext,
        std::{fmt, str::FromStr},
    };

    /// Serialize `AeCiphertext` values supporting `Display` trait
    pub fn serialize<S>(x: &PodAeCiphertext, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&x.to_string())
    }

    struct AeCiphertextVisitor;

    impl<'de> Visitor<'de> for AeCiphertextVisitor {
        type Value = PodAeCiphertext;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a FromStr type")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            FromStr::from_str(v).map_err(Error::custom)
        }
    }

    /// Deserialize `AeCiphertext` values from `str`
    pub fn deserialize<'de, D>(d: D) -> Result<PodAeCiphertext, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_str(AeCiphertextVisitor)
    }
}

/// Helper to serialize / deserialize `PodElGamalPubkey` values
pub mod elgamalpubkey_fromstr {
    use {
        serde::{
            de::{Error, Visitor},
            Deserializer, Serializer,
        },
        solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
        std::{fmt, str::FromStr},
    };

    /// Serialize `ElGamalPubkey` values supporting `Display` trait
    pub fn serialize<S>(x: &PodElGamalPubkey, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&x.to_string())
    }

    struct ElGamalPubkeyVisitor;

    impl<'de> Visitor<'de> for ElGamalPubkeyVisitor {
        type Value = PodElGamalPubkey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a FromStr type")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            FromStr::from_str(v).map_err(Error::custom)
        }
    }

    /// Deserialize `ElGamalPubkey` values from `str`
    pub fn deserialize<'de, D>(d: D) -> Result<PodElGamalPubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_str(ElGamalPubkeyVisitor)
    }
}

/// Helper to serialize / deserialize `PodElGamalCiphertext` values
pub mod elgamalciphertext_fromstr {
    use {
        serde::{
            de::{Error, Visitor},
            Deserializer, Serializer,
        },
        solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
        std::{fmt, str::FromStr},
    };

    /// Serialize `ElGamalCiphertext` values supporting `Display` trait
    pub fn serialize<S>(x: &PodElGamalCiphertext, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&x.to_string())
    }

    struct ElGamalCiphertextVisitor;

    impl<'de> Visitor<'de> for ElGamalCiphertextVisitor {
        type Value = PodElGamalCiphertext;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a FromStr type")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            FromStr::from_str(v).map_err(Error::custom)
        }
    }

    /// Deserialize `ElGamalCiphertext` values from `str`
    pub fn deserialize<'de, D>(d: D) -> Result<PodElGamalCiphertext, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_str(ElGamalCiphertextVisitor)
    }
}
