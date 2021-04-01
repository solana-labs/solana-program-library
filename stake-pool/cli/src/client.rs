use {
    bincode::deserialize,
    borsh::BorshDeserialize,
    solana_account_decoder::UiAccountEncoding,
    solana_client::{
        client_error::ClientError,
        rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    },
    solana_program::{program_pack::Pack, pubkey::Pubkey},
    spl_stake_pool::{
        borsh::try_from_slice_unchecked,
        stake_program,
        state::{StakePool, ValidatorList},
    },
};

type Error = Box<dyn std::error::Error>;

pub(crate) fn get_stake_pool(
    rpc_client: &RpcClient,
    pool_address: &Pubkey,
) -> Result<StakePool, Error> {
    let account_data = rpc_client.get_account_data(pool_address)?;
    let stake_pool = StakePool::try_from_slice(account_data.as_slice())
        .map_err(|err| format!("Invalid stake pool {}: {}", pool_address, err))?;
    Ok(stake_pool)
}

pub(crate) fn get_validator_list(
    rpc_client: &RpcClient,
    validator_list_address: &Pubkey,
) -> Result<ValidatorList, Error> {
    let account_data = rpc_client.get_account_data(validator_list_address)?;
    let validator_list = try_from_slice_unchecked::<ValidatorList>(&account_data.as_slice())
        .map_err(|err| format!("Invalid validator list {}: {}", validator_list_address, err))?;
    Ok(validator_list)
}

pub(crate) fn get_token_account(
    rpc_client: &RpcClient,
    token_account_address: &Pubkey,
    expected_token_mint: &Pubkey,
) -> Result<spl_token::state::Account, Error> {
    let account_data = rpc_client.get_account_data(token_account_address)?;
    let token_account = spl_token::state::Account::unpack_from_slice(account_data.as_slice())
        .map_err(|err| format!("Invalid token account {}: {}", token_account_address, err))?;

    if token_account.mint != *expected_token_mint {
        Err(format!(
            "Invalid token mint for {}, expected mint is {}",
            token_account_address, expected_token_mint
        )
        .into())
    } else {
        Ok(token_account)
    }
}

pub(crate) fn get_token_mint(
    rpc_client: &RpcClient,
    token_mint_address: &Pubkey,
) -> Result<spl_token::state::Mint, Error> {
    let account_data = rpc_client.get_account_data(token_mint_address)?;
    let token_mint = spl_token::state::Mint::unpack_from_slice(account_data.as_slice())
        .map_err(|err| format!("Invalid token mint {}: {}", token_mint_address, err))?;

    Ok(token_mint)
}

pub(crate) fn get_stake_state(
    rpc_client: &RpcClient,
    stake_address: &Pubkey,
) -> Result<stake_program::StakeState, Error> {
    let account_data = rpc_client.get_account_data(stake_address)?;
    let stake_state = deserialize(account_data.as_slice())
        .map_err(|err| format!("Invalid stake account {}: {}", stake_address, err))?;
    Ok(stake_state)
}

pub(crate) fn get_stake_accounts_by_withdraw_authority(
    rpc_client: &RpcClient,
    withdraw_authority: &Pubkey,
) -> Result<Vec<(Pubkey, u64, stake_program::StakeState)>, ClientError> {
    rpc_client
        .get_program_accounts_with_config(
            &stake_program::id(),
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::Memcmp(Memcmp {
                    offset: 44, // 44 is Withdrawer authority offset in stake account stake
                    bytes: MemcmpEncodedBytes::Binary(format!("{}", withdraw_authority)),
                    encoding: None,
                })]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..RpcAccountInfoConfig::default()
                },
            },
        )
        .map(|accounts| {
            accounts
                .into_iter()
                .filter_map(
                    |(address, account)| match deserialize(account.data.as_slice()) {
                        Ok(stake_state) => Some((address, account.lamports, stake_state)),
                        Err(err) => {
                            eprintln!("Invalid stake account data for {}: {}", address, err);
                            None
                        }
                    },
                )
                .collect()
        })
}
