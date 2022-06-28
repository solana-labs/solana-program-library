#[cfg(test)]
#[cfg(feature = "serde")]
mod tests_serde {
    use {
        solana_program::program_option::COption,
        solana_program_test::tokio, solana_sdk::pubkey::Pubkey, spl_token_2022::instruction,
        std::str::FromStr,
        anyhow::Result,
    };

    #[tokio::test]
    async fn token_program_serde() -> Result<()> {
        let inst = instruction::TokenInstruction::InitializeMint2 {
            decimals: 0,
            mint_authority: Pubkey::from_str("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM")?,
            freeze_authority: COption::Some(Pubkey::from_str("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh")?),
        };

        let serialized = serde_json::to_string(&inst)?;
        assert_eq!(&serialized, "{\"InitializeMint2\":{\"decimals\":0,\"mint_authority\":\"4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM\",\"freeze_authority\":\"8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh\"}}");

        let _ = serde_json::from_str::<instruction::TokenInstruction>(&serialized)?;
        Ok(())
    }

    #[tokio::test]
    async fn token_program_serde_with_none() -> Result<()> {
        let inst = instruction::TokenInstruction::InitializeMintCloseAuthority {
            close_authority: COption::None, 
        };

        let serialized = serde_json::to_string(&inst)?;
        assert_eq!(&serialized, "{\"InitializeMintCloseAuthority\":{\"close_authority\":null}}");

        let _ = serde_json::from_str::<instruction::TokenInstruction>(&serialized)?;
        Ok(())
    }
}
