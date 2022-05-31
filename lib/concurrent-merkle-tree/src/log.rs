#[cfg(feature = "sol-log")]
use solana_program::{log::sol_log_compute_units, msg};

#[cfg(feature = "sol-log")]
macro_rules! solana_logging {
    ($message:literal, $($arg:tt)*) => {
        msg!($message, $($arg)*);
    };
    ($message:literal) => {
        println!($message);
    };
}

#[cfg(not(feature = "sol-log"))]
macro_rules! solana_logging {
    ($message:literal, $($arg:tt)*) => {
        println!($message, $($arg)*);
    };
    ($message:literal) => {
        println!($message);
    };
}

#[cfg(feature = "sol-log")]
macro_rules! log_compute {
    () => {
        sol_log_compute_units();
    };
}

#[cfg(not(feature = "sol-log"))]
macro_rules! log_compute {
    () => {};
}
