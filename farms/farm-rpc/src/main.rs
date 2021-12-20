//! Solana Farms RPC Backend.

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rocket;

mod config;
mod git_token;
mod json_rpc;

use {
    clap::{crate_description, crate_name, App, Arg},
    log::{debug, info},
    solana_clap_utils::input_validators::is_url,
    url::Url,
};

#[rocket::main]
async fn main() {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(solana_version::version!())
        .arg(
            Arg::with_name("config_file")
                .short("C")
                .long("config-file")
                .value_name("PATH")
                .takes_value(true)
                .help("Configuration file to use"),
        )
        .arg(
            Arg::with_name("save_config")
                .short("S")
                .long("save-config")
                .value_name("PATH")
                .takes_value(true)
                .help("Write current config to a file"),
        )
        .arg(
            Arg::with_name("log_level")
                .short("L")
                .long("log-level")
                .takes_value(true)
                .help("Log verbosity level (debug, info, warning, error)")
                .validator(|p| {
                    let allowed = ["debug", "info", "warning", "error"];
                    if allowed.contains(&p.as_str()) {
                        Ok(())
                    } else {
                        Err(String::from("Must be one of: debug, info, warning, error"))
                    }
                }),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .short("u")
                .long("json-rpc-url")
                .value_name("STR")
                .takes_value(true)
                .validator(is_url)
                .help("URL for JSON RPC service"),
        )
        .arg(
            Arg::with_name("websocket_url")
                .short("w")
                .long("websocket-url")
                .value_name("STR")
                .takes_value(true)
                .validator(is_url)
                .help("URL for Websocket service"),
        )
        .arg(
            Arg::with_name("max_threads")
                .short("m")
                .long("max-threads")
                .value_name("NUM")
                .takes_value(true)
                .validator(|p| match p.parse::<u32>() {
                    Err(_) => Err(String::from("Must be unsigned integer")),
                    Ok(_) => Ok(()),
                })
                .help("Max threads for incoming connections"),
        )
        .arg(
            Arg::with_name("token_list_url")
                .short("t")
                .long("token-list-url")
                .value_name("STR")
                .takes_value(true)
                .validator(is_url)
                .help("URL for Solana's tokens list"),
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
        .get_matches();

    // set log verbosity level
    let mut log_level = String::from("solana=info");
    if let Some(level) = matches.value_of("log_level") {
        log_level = "solana=".to_string() + level;
    }
    solana_logger::setup_with_default(log_level.as_str());

    info!("Loading configuration...");

    // start with default config settings
    let mut config: config::Config = Default::default();
    // if config path is explicitly specified, load config from there and stop
    // on error. Otherwise try to load from default path and allow to proceed
    // with default config if file not found.
    if let Some(config_file) = matches.value_of("config_file") {
        config.load(config_file).unwrap();
    } else if let Some(ref config_file) = *config::CONFIG_FILE {
        let _ = config.load(config_file);
    }
    // override loaded or default params with explicit cmd line arguments
    if let Some(json_rpc_url) = matches.value_of("json_rpc_url") {
        config.json_rpc_url = json_rpc_url.to_string();
    }
    if let Some(websocket_url) = matches.value_of("websocket_url") {
        config.websocket_url = websocket_url.to_string();
    }
    if let Some(max_threads) = matches.value_of("max_threads") {
        config.max_threads = max_threads.parse().unwrap();
    }
    if let Some(token_list_url) = matches.value_of("token_list_url") {
        config.token_list_url = token_list_url.to_string();
    }
    if let Some(farm_client_url) = matches.value_of("farm_client_url") {
        config.farm_client_url = farm_client_url.to_string();
    }
    // save config to a file
    if let Some(config_file) = matches.value_of("save_config") {
        config.save(config_file).unwrap();
        info!("Configuration saved to: {}", config_file);
    }

    debug!("json_rpc_url: {}", config.json_rpc_url);
    debug!("websocket_url: {}", config.websocket_url);
    debug!("farm_client_url: {}", config.farm_client_url);
    debug!("max_threads: {}", config.max_threads);

    info!("Starting JSON RPC on {}", config.json_rpc_url);
    let parsed_url: Url = config.json_rpc_url.parse().unwrap();
    let figment = rocket::Config::figment()
        .merge(("port", parsed_url.port().unwrap()))
        .merge(("address", parsed_url.host_str().unwrap()))
        .merge(("workers", config.max_threads))
        .merge(("ident", "Farms JSON RPC"));

    let json_rpc = rocket::custom(figment)
        .attach(json_rpc::stage(&config).await)
        .launch();
    json_rpc.await.unwrap();

    info!("Shutting down...");
}
