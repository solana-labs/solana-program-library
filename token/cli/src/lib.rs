mod bench;
pub mod clap_app;
pub mod command;
pub mod config;
mod encryption_keypair;
mod output;
mod sort;

fn print_error_and_exit<T, E: std::fmt::Display>(e: E) -> T {
    eprintln!("error: {}", e);
    std::process::exit(1)
}
