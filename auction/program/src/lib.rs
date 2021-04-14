mod entrypoint;
mod errors;
mod processor;
mod utils;

/// Prefix used in PDA derivations to avoid collisions.
const PREFIX: &str = "auction";
