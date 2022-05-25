//! Initialize a new user for a Raydium farm instruction

use {
    solana_farm_sdk::{
        farm::{Farm, FarmRoute},
        id::main_router,
        program::{account, pda, protocol::raydium},
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn user_init(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Processing AmmInstruction::UserInit");

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        funding_account,
        user_account,
        user_info_account,
        farm_metadata,
        _system_program
        ] = accounts
    {
        if account::exists(user_info_account)? {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if farm_metadata.owner != &main_router::id() {
            msg!("Error: Invalid Farm metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        let farm = Farm::unpack(&farm_metadata.try_borrow_data()?)?;

        if !raydium::check_stake_program_id(&farm.farm_program_id) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let farm_id = match farm.route {
            FarmRoute::Raydium { farm_id, .. } => farm_id,
            _ => {
                return Err(ProgramError::InvalidArgument);
            }
        };
        let seeds: &[&[u8]] = &[
            b"Miner",
            &farm_id.to_bytes(),
            &user_account.key.to_bytes(),
        ];

        pda::init_system_account(
            funding_account,
            user_info_account,
            &farm.farm_program_id,
            &farm.router_program_id,
            seeds,
            if farm.version >= 4 {
                raydium::RaydiumUserStakeInfoV4::LEN
            } else {
                raydium::RaydiumUserStakeInfo::LEN
            },
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::UserInit complete");
    Ok(())
}
