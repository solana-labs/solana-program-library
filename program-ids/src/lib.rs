//! SPL Program IDs

macro_rules! delcare_id_mod {
    ($mod_name:ident, $id_bs58:literal) => {
        pub mod $mod_name {
            ::solana_program::declare_id!($id_bs58);
        }
    }
}

delcare_id_mod!(account_compression, "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK");

#[cfg(test)]
mod tests {
    use super::account_compression;

    #[test]
    fn test_declare_id_mod() {
        assert_eq!(account_compression::ID.to_string(), "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK");
    }
}
