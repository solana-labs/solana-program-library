pub mod instruction;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;
