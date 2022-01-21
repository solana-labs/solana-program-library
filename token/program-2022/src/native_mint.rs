//! The Mint that represents the native token

/// There are 10^9 lamports in one SOL
pub const DECIMALS: u8 = 9;

// The Mint for native SOL Token accounts
solana_program::declare_id!("So11111111111111111111111111111111111111112");

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::native_token::*;

    #[test]
    fn test_decimals() {
        assert!(
            (lamports_to_sol(42) - crate::amount_to_ui_amount(42, DECIMALS)).abs() < f64::EPSILON
        );
        assert_eq!(
            sol_to_lamports(42.),
            crate::ui_amount_to_amount(42., DECIMALS)
        );
    }
}
