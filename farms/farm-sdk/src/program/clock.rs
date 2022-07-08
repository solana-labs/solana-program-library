//! Timing functions

use {
    crate::math,
    solana_program::{clock::UnixTimestamp, program_error::ProgramError, sysvar, sysvar::Sysvar},
};

pub fn get_time() -> Result<UnixTimestamp, ProgramError> {
    Ok(sysvar::clock::Clock::get()?.unix_timestamp)
}

pub fn get_time_as_u64() -> Result<u64, ProgramError> {
    math::checked_as_u64(sysvar::clock::Clock::get()?.unix_timestamp)
}

pub fn get_slot() -> Result<u64, ProgramError> {
    Ok(sysvar::clock::Clock::get()?.slot)
}

/*
pub fn unix_timestamp_to_string(unix_timestamp: UnixTimestamp) -> String {
    match NaiveDateTime::from_timestamp_opt(unix_timestamp, 0) {
        Some(ndt) => DateTime::<Utc>::from_utc(ndt, Utc).to_rfc3339_opts(SecondsFormat::Secs, true),
        None => format!("UnixTimestamp {}", unix_timestamp),
    }
}
*/
