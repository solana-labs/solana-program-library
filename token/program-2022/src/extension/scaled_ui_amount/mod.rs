#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    solana_program::program_error::ProgramError,
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

/// Scaled UI amount extension instructions
pub mod instruction;

/// Scaled UI amount extension processor
pub mod processor;

/// `f64` type that can be used in `Pod`s
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(from = "f64", into = "f64"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodF64(pub [u8; 8]);
impl PodF64 {
    fn from_primitive(n: f64) -> Self {
        Self(n.to_le_bytes())
    }
}
impl From<f64> for PodF64 {
    fn from(n: f64) -> Self {
        Self::from_primitive(n)
    }
}
impl From<PodF64> for f64 {
    fn from(pod: PodF64) -> Self {
        Self::from_le_bytes(pod.0)
    }
}

/// Scaled UI amount extension data for mints
#[repr(C)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ScaledUiAmountConfig {
    /// Authority that can set the scaling amount and authority
    pub authority: OptionalNonZeroPubkey,
    /// Amount to multiply raw amounts by, outside of the decimal
    pub scale: PodF64,
}
impl ScaledUiAmountConfig {
    fn total_scale(&self, decimals: u8) -> f64 {
        f64::from(self.scale) / 10_f64.powi(decimals as i32)
    }

    /// Convert a raw amount to its UI representation using the given decimals
    /// field Excess zeroes or unneeded decimal point are trimmed.
    pub fn amount_to_ui_amount(&self, amount: u64, decimals: u8) -> Option<String> {
        let scaled_amount = (amount as f64) * self.total_scale(decimals);
        Some(scaled_amount.to_string())
    }

    /// Try to convert a UI representation of a token amount to its raw amount
    /// using the given decimals field
    pub fn try_ui_amount_into_amount(
        &self,
        ui_amount: &str,
        decimals: u8,
    ) -> Result<u64, ProgramError> {
        let scaled_amount = ui_amount
            .parse::<f64>()
            .map_err(|_| ProgramError::InvalidArgument)?;
        let amount = scaled_amount / self.total_scale(decimals);
        if amount > (u64::MAX as f64) || amount < (u64::MIN as f64) || amount.is_nan() {
            Err(ProgramError::InvalidArgument)
        } else {
            // this is important, if you round earlier, you'll get wrong "inf"
            // answers
            Ok(amount.round() as u64)
        }
    }
}
impl Extension for ScaledUiAmountConfig {
    const TYPE: ExtensionType = ExtensionType::ScaledUiAmount;
}

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    const TEST_DECIMALS: u8 = 2;

    #[test]
    fn specific_amount_to_ui_amount() {
        // 5x
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: PodF64::from(5.0),
        };
        let ui_amount = config.amount_to_ui_amount(1, 0).unwrap();
        assert_eq!(ui_amount, "5");
        // with 1 decimal place
        let ui_amount = config.amount_to_ui_amount(1, 1).unwrap();
        assert_eq!(ui_amount, "0.5");
        // with 10 decimal places
        let ui_amount = config.amount_to_ui_amount(1, 10).unwrap();
        assert_eq!(ui_amount, "0.0000000005");

        // huge amount with 10 decimal places
        let ui_amount = config.amount_to_ui_amount(10_000_000_000, 10).unwrap();
        assert_eq!(ui_amount, "5");

        // huge values
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: PodF64::from(f64::MAX),
        };
        let ui_amount = config.amount_to_ui_amount(u64::MAX, 0).unwrap();
        assert_eq!(ui_amount, "inf");
    }

    #[test]
    fn specific_ui_amount_to_amount() {
        // constant 5x
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 5.0.into(),
        };
        let amount = config.try_ui_amount_into_amount("5.0", 0).unwrap();
        assert_eq!(1, amount);
        // with 1 decimal place
        let amount = config.try_ui_amount_into_amount("0.500000000", 1).unwrap();
        assert_eq!(amount, 1);
        // with 10 decimal places
        let amount = config
            .try_ui_amount_into_amount("0.00000000050000000000000000", 10)
            .unwrap();
        assert_eq!(amount, 1);

        // huge amount with 10 decimal places
        let amount = config
            .try_ui_amount_into_amount("5.0000000000000000", 10)
            .unwrap();
        assert_eq!(amount, 10_000_000_000);

        // huge values
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 5.0.into(),
        };
        let amount = config
            .try_ui_amount_into_amount("92233720368547758075", 0)
            .unwrap();
        assert_eq!(amount, u64::MAX);
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: f64::MAX.into(),
        };
        // scientific notation "e"
        let amount = config
            .try_ui_amount_into_amount("1.7976931348623157e308", 0)
            .unwrap();
        assert_eq!(amount, 1);
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 9.745314011399998e288.into(),
        };
        let amount = config
            .try_ui_amount_into_amount("1.7976931348623157e308", 0)
            .unwrap();
        assert_eq!(amount, u64::MAX);
        // scientific notation "E"
        let amount = config
            .try_ui_amount_into_amount("1.7976931348623157E308", 0)
            .unwrap();
        assert_eq!(amount, u64::MAX);

        // this is unfortunate, but underflows can happen due to floats
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 1.0.into(),
        };
        assert_eq!(
            u64::MAX,
            config
                .try_ui_amount_into_amount("18446744073709551616", 0)
                .unwrap() // u64::MAX + 1
        );

        // overflow u64 fail
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 0.1.into(),
        };
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            config.try_ui_amount_into_amount("18446744073709551615", 0) // u64::MAX + 1
        );

        for fail_ui_amount in ["-0.0000000000000000000001", "inf", "-inf", "NaN"] {
            assert_eq!(
                Err(ProgramError::InvalidArgument),
                config.try_ui_amount_into_amount(fail_ui_amount, 0)
            );
        }
    }

    #[test]
    fn specific_amount_to_ui_amount_no_scale() {
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 1.0.into(),
        };
        for (amount, expected) in [(23, "0.23"), (110, "1.1"), (4200, "42"), (0, "0")] {
            let ui_amount = config.amount_to_ui_amount(amount, TEST_DECIMALS).unwrap();
            assert_eq!(ui_amount, expected);
        }
    }

    #[test]
    fn specific_ui_amount_to_amount_no_scale() {
        let config = ScaledUiAmountConfig {
            authority: OptionalNonZeroPubkey::default(),
            scale: 1.0.into(),
        };
        for (ui_amount, expected) in [
            ("0.23", 23),
            ("0.20", 20),
            ("0.2000", 20),
            (".2", 20),
            ("1.1", 110),
            ("1.10", 110),
            ("42", 4200),
            ("42.", 4200),
            ("0", 0),
        ] {
            let amount = config
                .try_ui_amount_into_amount(ui_amount, TEST_DECIMALS)
                .unwrap();
            assert_eq!(expected, amount);
        }

        // this is invalid with normal mints, but rounding for this mint makes it ok
        let amount = config
            .try_ui_amount_into_amount("0.111", TEST_DECIMALS)
            .unwrap();
        assert_eq!(11, amount);

        // fail if invalid ui_amount passed in
        for ui_amount in ["", ".", "0.t"] {
            assert_eq!(
                Err(ProgramError::InvalidArgument),
                config.try_ui_amount_into_amount(ui_amount, TEST_DECIMALS),
            );
        }
    }

    proptest! {
        #[test]
        fn amount_to_ui_amount(
            scale in 0f64..=f64::MAX,
            amount in 0..=u64::MAX,
            decimals in 0u8..20u8,
        ) {
            let config = ScaledUiAmountConfig {
                authority: OptionalNonZeroPubkey::default(),
                scale: scale.into(),
            };
            let ui_amount = config.amount_to_ui_amount(amount, decimals);
            assert!(ui_amount.is_some());
        }
    }
}
