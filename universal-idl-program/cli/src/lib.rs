use anyhow::Result;
use clap::Parser;
use solana_program::pubkey::Pubkey;

mod instruction_helpers;
use instruction_helpers::*;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(flatten)]
    pub cfg_override: CliOverrides,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    /// Creates a program's IDL account
    CreateIdl {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "payer")]
        payer: String,
        #[clap(long = "authority")]
        program_authority: String,
        #[clap(long = "idl")]
        filepath: String,
    },
    /// Sets an authority for a frozen program.
    /// Only meta authorities can use this
    DeclareFrozenAuthority {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "new-authority")]
        new_program_authority: Pubkey,
        #[clap(long = "authority")]
        payer: String,
    },
    /// Closes a program's buffer account
    CloseBuffer {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "recipient")]
        recipient: Pubkey,
        #[clap(long = "authority")]
        authority_filepath: String,
    },
    /// Closes a program's IDL account
    CloseIdl {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "recipient")]
        recipient: Pubkey,
        #[clap(long = "authority")]
        authority_filepath: String,
    },
    /// Writes an IDL into a buffer account. This can be used with SetBuffer
    /// to perform an upgrade. This will overwrite the buffer account.
    WriteBuffer {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "payer")]
        payer_filepath: String,
        #[clap(long = "authority")]
        authority_filepath: String,
        #[clap(long = "idl")]
        filepath: String,
    },
    /// Sets a new IDL buffer for the program.
    SetBuffer {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "payer")]
        payer_filepath: String,
        #[clap(long = "authority")]
        authority_filepath: String,
    },
    /// Upgrades the IDL to the new file. An alias for first writing and then
    /// then setting the idl buffer account.
    Upgrade {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "payer")]
        payer_filepath: String,
        #[clap(long = "authority")]
        authority_filepath: String,
        #[clap(long = "idl")]
        filepath: String,
    },
    /// Sets a new authority on the IDL account.
    SetAuthority {
        /// Program to change the IDL authority.
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        /// New authority of the IDL account.
        #[clap(long = "new-authority")]
        new_authority: Pubkey,
        /// Filepath to the authority on the IDL account
        #[clap(long = "authority")]
        authority_filepath: String,
    },
    /// Command to remove the ability to modify the IDL account. This should
    /// likely be used in conjection with eliminating an "upgrade authority" on
    /// the program.
    EraseAuthority {
        #[clap(short = 'p', long = "program")]
        program_id: Pubkey,
        #[clap(long = "authority")]
        authority_filepath: String,
    },
    /// Fetches an IDL for the given address from a cluster.
    /// The address can be a program, IDL account, or IDL buffer.
    Fetch { address: Pubkey },
}

pub fn entry(opts: Opts) -> Result<()> {
    match opts.command {
        Command::CreateIdl {
            program_id,
            payer,
            program_authority,
            filepath,
        } => {
            instruction_helpers::create_idl(
                opts.cfg_override,
                program_id,
                &payer,
                &program_authority,
                filepath,
            )?;
        }
        Command::DeclareFrozenAuthority {
            program_id,
            new_program_authority,
            payer,
        } => {
            instruction_helpers::declare_frozen_authority(
                opts.cfg_override,
                program_id,
                new_program_authority,
                payer,
            )?;
        }
        Command::CloseBuffer {
            program_id,
            recipient,
            authority_filepath,
        } => {
            instruction_helpers::close_account(
                opts.cfg_override,
                program_id,
                IdlAccountType::Buffer,
                recipient,
                &authority_filepath,
            )?;
        }
        Command::CloseIdl {
            program_id,
            recipient,
            authority_filepath,
        } => {
            instruction_helpers::close_account(
                opts.cfg_override,
                program_id,
                IdlAccountType::Idl,
                recipient,
                &authority_filepath,
            )?;
        }
        Command::EraseAuthority {
            program_id,
            authority_filepath,
        } => {
            instruction_helpers::set_authority(
                opts.cfg_override,
                program_id,
                Pubkey::default(),
                &authority_filepath,
            )?;
        }
        Command::WriteBuffer {
            program_id,
            payer_filepath,
            authority_filepath,
            filepath,
        } => {
            instruction_helpers::write_buffer(
                opts.cfg_override,
                program_id,
                &payer_filepath,
                &authority_filepath,
                &filepath,
            )?;
        }
        Command::SetBuffer {
            program_id,
            payer_filepath,
            authority_filepath,
        } => {
            instruction_helpers::set_buffer(
                opts.cfg_override,
                program_id,
                &payer_filepath,
                &authority_filepath,
            )?;
        }
        Command::Upgrade {
            program_id,
            payer_filepath,
            authority_filepath,
            filepath,
        } => {
            instruction_helpers::upgrade(
                opts.cfg_override,
                program_id,
                &payer_filepath,
                &authority_filepath,
                &filepath,
            )?;
        }
        Command::SetAuthority {
            program_id,
            new_authority,
            authority_filepath,
        } => {
            instruction_helpers::set_authority(
                opts.cfg_override,
                program_id,
                new_authority,
                &authority_filepath,
            )?;
        }
        Command::Fetch { address } => {
            idl_fetch(opts.cfg_override, address)?;
        }
    }
    Ok(())
}
