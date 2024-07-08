//! SPL Program IDs

macro_rules! declare_id_mod {
    ($mod_name:ident, $id_bs58:literal) => {
        pub mod $mod_name {
            ::solana_program::declare_id!($id_bs58);
        }
    };
}

declare_id_mod!(
    spl_account_compression,
    "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"
);
declare_id_mod!(spl_noop, "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");
declare_id_mod!(
    spl_associated_token_account,
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);
declare_id_mod!(
    spl_binary_option,
    "betw959P4WToez4DkuXwNsJszqbpe3HuY56AcG5yevx"
);
declare_id_mod!(
    spl_bianry_oracle_pair,
    "Fd7btgySsrjuo25CJCj7oE7VPMyezDhnx7pZkj2v69Nk"
);
declare_id_mod!(
    spl_feature_proposal,
    "Feat1YXHhH6t1juaWF74WLcfv4XoNocjXA6sPWHNgAse"
);
declare_id_mod!(
    spl_instruction_padding,
    "iXpADd6AW1k5FaaXum5qHbSqyd7TtoN6AD7suVa83MF"
);
declare_id_mod!(spl_math, "Math111111111111111111111111111111111111111");
declare_id_mod!(
    spl_managed_token,
    "mTok58Lg4YfcmwqyrDHpf7ogp599WRhzb6PxjaBqAxS"
);
declare_id_mod!(
    spl_name_service,
    "namesLPneVptA9Z5rqUDD9tMTWEJwofgaYwp8cawRkX"
);
declare_id_mod!(spl_record, "recr1L3PCGKLbckBqMNcJhuuyU1zgo8nBhfLVsJNwr5");
declare_id_mod!(
    spl_shared_memory,
    "shmem4EWT2sPdVGvTZCzXXRAURL9G5vpPxNwSeKhHUL"
);
declare_id_mod!(
    spl_single_pool,
    "SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE"
);
declare_id_mod!(
    spl_stake_pool,
    "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy"
);
declare_id_mod!(
    spl_token_lending,
    "6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH"
);
declare_id_mod!(
    spl_token_swap,
    "SwapsVeCiPHMUAtzQWZw7RjsKjgCjhwU55QGu4U1Szw"
);
declare_id_mod!(
    spl_token_upgrade,
    "TkupDoNseygccBCjSsrSpMccjwHfTYwcrjpnDSrFDhC"
);
declare_id_mod!(
    spl_token_wrap,
    "TwRapQCDhWkZRrDaHfZGuHxkZ91gHDRkyuzNqeU5MgR"
);

pub mod spl_memo {
    /// Legacy symbols from Memo v1
    pub mod v1 {
        solana_program::declare_id!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");
    }
    solana_program::declare_id!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
}

pub mod spl_token {
    pub mod native_mint {
        solana_program::declare_id!("So11111111111111111111111111111111111111112");
    }
    solana_program::declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
}

pub mod spl_token_2022 {
    pub mod native_mint {
        solana_program::declare_id!("9pan9bMn5HatX4EJdBwg9VgCa7Uz5HL8N1m5D3NdXejP");
    }
    solana_program::declare_id!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
}

#[cfg(test)]
mod tests {
    use super::spl_account_compression;

    #[test]
    fn test_declare_id_mod() {
        assert_eq!(
            spl_account_compression::ID.to_string(),
            "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"
        );
    }
}
