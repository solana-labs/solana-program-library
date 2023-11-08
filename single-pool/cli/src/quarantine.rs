// XXX this file will be deleted and replaced with a stake program client once i
// write one

use {
    crate::config::*,
    solana_sdk::{
        instruction::Instruction,
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        stake::{
            self,
            state::{Meta, Stake, StakeStateV2},
        },
        system_instruction,
        sysvar::{self, rent::Rent},
    },
};

pub async fn get_rent(config: &Config) -> Result<Rent, Error> {
    let rent_data = config
        .program_client
        .get_account(sysvar::rent::id())
        .await?
        .unwrap();
    let rent = bincode::deserialize::<Rent>(&rent_data.data)?;

    Ok(rent)
}

pub async fn get_minimum_delegation(config: &Config) -> Result<u64, Error> {
    Ok(std::cmp::max(
        config.rpc_client.get_stake_minimum_delegation().await?,
        LAMPORTS_PER_SOL,
    ))
}

pub async fn get_stake_info(
    config: &Config,
    stake_account_address: &Pubkey,
) -> Result<Option<(Meta, Stake)>, Error> {
    if let Some(stake_account) = config
        .program_client
        .get_account(*stake_account_address)
        .await?
    {
        match bincode::deserialize::<StakeStateV2>(&stake_account.data)? {
            StakeStateV2::Stake(meta, stake, _) => Ok(Some((meta, stake))),
            StakeStateV2::Initialized(_) => {
                Err(format!("Stake account {} is undelegated", stake_account_address).into())
            }
            StakeStateV2::Uninitialized => {
                Err(format!("Stake account {} is uninitialized", stake_account_address).into())
            }
            StakeStateV2::RewardsPool => unimplemented!(),
        }
    } else {
        Ok(None)
    }
}

pub async fn create_uninitialized_stake_account_instruction(
    config: &Config,
    payer: &Pubkey,
    stake_account: &Pubkey,
) -> Result<Instruction, Error> {
    let rent_amount = config
        .program_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<StakeStateV2>())
        .await?;

    Ok(system_instruction::create_account(
        payer,
        stake_account,
        rent_amount,
        std::mem::size_of::<StakeStateV2>() as u64,
        &stake::program::id(),
    ))
}
