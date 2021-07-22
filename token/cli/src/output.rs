use crate::config::Config;
use solana_cli_output::{OutputFormat, QuietDisplay, VerboseDisplay};

pub(crate) fn println_display(config: &Config, message: String) {
    match config.output_format {
        OutputFormat::Display | OutputFormat::DisplayVerbose => {
            println!("{}", message);
        }
        _ => {}
    }
}
