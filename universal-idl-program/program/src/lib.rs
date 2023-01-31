use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{self, entrypoint::ProgramResult, msg};

use shank::ShankInstruction;
use solana_security_txt::security_txt;

pub mod account_checks;
pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

solana_program::declare_id!("uipLuk57b21BUNutsX2kVxCyTXZeBmfyd4dswaRjWaL");

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Universal IDL Program",
    project_url: "?",
    contacts: "email:noah.gundotra@solana.com,",
    policy: "?",
    preferred_languages: "en",
    source_code: "https://github.com/solana-labs/solana-program-library"
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Clone, ShankInstruction)]
pub enum IdlProgramInstruction {
    #[account(0, signer, writable, name = "payer")]
    #[account(1, signer, name = "program_authority")]
    #[account(2, writable, name = "idl_account")]
    #[account(3, name = "program")]
    #[account(4, name = "program_data_or_frozen_authority")]
    #[account(5, name = "system_program")]
    CreateIdl,

    #[account(0, name = "program")]
    #[account(1, writable, name = "frozen_program_authority")]
    #[account(2, signer, name = "meta_authority")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    DeclareFrozenAuthority(DeclareFrozenAuthorityArgs),

    #[account(0, signer, writable, name = "payer")]
    #[account(1, name = "idl_account")]
    #[account(2, signer, name = "authority")]
    #[account(3, writable, name = "buffer_account")]
    #[account(4, name = "program")]
    #[account(5, name = "system_program")]
    CreateBuffer,

    #[account(0, signer, writable, name = "wallet")]
    #[account(1, signer, name = "authority")]
    #[account(2, writable, name = "idl_account")]
    #[account(3, writable, name = "buffer_account")]
    #[account(4, name = "program")]
    SetBuffer,

    #[account(0, writable, name = "idl_account")]
    #[account(1, name = "program")]
    #[account(2, signer, name = "authority")]
    SetAuthority(SetAuthorityArgs),

    #[account(0, writable, name = "idl_account")]
    #[account(1, writable, name = "recipient")]
    #[account(2, signer, name = "authority")]
    Close,

    #[account(0, writable, name = "buffer_account")]
    #[account(1, writable, signer, name = "payer")]
    #[account(2, signer, name = "authority")]
    #[account(3, name = "program")]
    #[account(4, name = "system_program")]
    Extend(ExtendArgs),
}

pub fn process_instruction(
    _program_id: &solana_program::pubkey::Pubkey,
    accounts: &[solana_program::account_info::AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = IdlProgramInstruction::try_from_slice(instruction_data)?;

    match instruction {
        IdlProgramInstruction::CreateIdl => {
            msg!("IdlProgramInstruction::CreateIdl");
            let ctx = CreateIdlCtx::load(accounts)?;
            instructions::create_idl::handler(ctx)
        }
        IdlProgramInstruction::DeclareFrozenAuthority(args) => {
            msg!("IdlProgramInstruction::DeclareFrozenAuthority");
            let ctx = DeclareFrozenAuthorityCtx::load(accounts)?;
            instructions::declare_frozen_authority::handler(ctx, args)
        }
        IdlProgramInstruction::CreateBuffer => {
            msg!("IdlProgramInstruction::CreateBuffer");
            let ctx = CreateBufferCtx::load(accounts)?;
            instructions::create_buffer::handler(ctx)
        }
        IdlProgramInstruction::Close => {
            msg!("IdlProgramInstruction::Close");
            let ctx = CloseCtx::load(accounts)?;
            instructions::close::handler(ctx)
        }
        IdlProgramInstruction::SetBuffer => {
            msg!("IdlProgramInstruction::SetBuffer");
            let ctx = SetBufferCtx::load(accounts)?;
            instructions::set_buffer::handler(ctx)
        }
        IdlProgramInstruction::SetAuthority(args) => {
            msg!("IdlProgramInstruction::SetAuthority");
            let ctx = SetAuthorityCtx::load(accounts)?;
            instructions::set_authority::handler(ctx, args)
        }
        IdlProgramInstruction::Extend(args) => {
            msg!("IdlProgramInstruction::Extend");
            let ctx = ExtendCtx::load(accounts)?;
            instructions::extend::handler(ctx, args)
        }
    }
}
