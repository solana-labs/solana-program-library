//! Funds performance stats collection program

mod collector;
mod fund_stats;

use {
    clap::{crate_description, crate_name, App, Arg},
    log::{error, info},
    solana_clap_utils::input_validators::is_url,
    std::{thread, time::Duration},
};

fn main() {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(solana_version::version!())
        .arg(
            Arg::with_name("log_level")
                .short("L")
                .long("log-level")
                .takes_value(true)
                .default_value("info")
                .global(true)
                .help("Log verbosity level")
                .possible_values(&["debug", "info", "warning", "error"])
                .hide_possible_values(false),
        )
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("update_interval_sec")
                .short("i")
                .long("update-interval-sec")
                .value_name("SEC")
                .takes_value(true)
                .default_value("900")
                .validator(|p| match p.parse::<u32>() {
                    Err(_) => Err(String::from("Must be unsigned integer")),
                    Ok(_) => Ok(()),
                })
                .help("Stats update interval in seconds"),
        )
        .arg(
            Arg::with_name("farm_client_url")
                .short("f")
                .long("farm-client-url")
                .value_name("STR")
                .takes_value(true)
                .validator(is_url)
                .help("RPC URL to use with Farm Client"),
        )
        .arg(
            Arg::with_name("sqlite_db_path")
                .short("s")
                .long("sqlite-db-path")
                .value_name("STR")
                .takes_value(true)
                .required(true)
                .help("RPC URL to use with Farm Client"),
        )
        .get_matches();

    // set log verbosity level
    let log_level = "solana=".to_string() + matches.value_of("log_level").unwrap();
    solana_logger::setup_with_default(log_level.as_str());

    // load config params
    let farm_client_url = if let Some(farm_client_url) = matches.value_of("farm_client_url") {
        farm_client_url.to_string()
    } else {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            match solana_cli_config::Config::load(config_file) {
                Err(e) => {
                    panic!("Failed to load config file \"{}\":{}", config_file, e);
                }
                Ok(config) => config,
            }
        } else {
            solana_cli_config::Config::default()
        };
        cli_config.json_rpc_url
    };

    loop {
        if let Err(e) = collector::collect(
            &farm_client_url,
            matches.value_of("sqlite_db_path").unwrap(),
            matches
                .value_of("update_interval_sec")
                .unwrap()
                .parse()
                .unwrap(),
        ) {
            error!("Error: {}", e);
            info!("Waiting for 20 secs before restarting the process...");
            thread::sleep(Duration::from_secs(20));
        }
    }
}
