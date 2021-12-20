//! Timing functions

use std::cmp;
use {
    crate::vault_info::VaultInfo,
    solana_farm_sdk::math,
    solana_program::{
        clock::UnixTimestamp, entrypoint::ProgramResult, msg, program_error::ProgramError, sysvar,
        sysvar::Sysvar,
    },
};

pub fn get_time() -> Result<UnixTimestamp, ProgramError> {
    Ok(sysvar::clock::Clock::get()?.unix_timestamp)
}

pub fn get_time_as_u64() -> Result<u64, ProgramError> {
    math::checked_as_u64(sysvar::clock::Clock::get()?.unix_timestamp)
}

pub fn check_min_crank_interval(vault_info: &VaultInfo) -> ProgramResult {
    let min_crank_interval = vault_info.get_min_crank_interval()?;
    if min_crank_interval == 0 {
        return Ok(());
    }
    let last_crank_time = vault_info.get_crank_time()?;
    let cur_time = cmp::max(get_time()?, last_crank_time);
    if cur_time < last_crank_time.wrapping_add(min_crank_interval) {
        msg!(
            "Error: Too early, please wait for the additional {} sec",
            last_crank_time
                .wrapping_add(min_crank_interval)
                .wrapping_sub(cur_time)
        );
        Err(ProgramError::Custom(309))
    } else {
        Ok(())
    }
}

/*
pub fn unix_timestamp_to_string(unix_timestamp: UnixTimestamp) -> String {
    match NaiveDateTime::from_timestamp_opt(unix_timestamp, 0) {
        Some(ndt) => DateTime::<Utc>::from_utc(ndt, Utc).to_rfc3339_opts(SecondsFormat::Secs, true),
        None => format!("UnixTimestamp {}", unix_timestamp),
    }
}
*/
