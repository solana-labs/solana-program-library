//! serialization module

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
