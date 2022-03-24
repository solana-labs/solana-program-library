use anchor_lang::prelude::*;

declare_id!("FGQLqP4smSphjS1WRamE3zEezL8tCFZrsu4Wg5z2Kq4r");

#[program]
pub mod record_anchor {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("RecordInstruction::Initialize");

        let data_info = &mut ctx.accounts.record_account;
        let authority_info = *ctx.accounts.authority.key;

        data_info.authority = authority_info;
        data_info.version = CURRENT_VERSION;

        Ok(())
    }

    pub fn set_authority(ctx: Context<SetAuthority>) -> Result<()> {
        msg!("RecordInstruction::SetAuthority");
        let data_info = &mut ctx.accounts.record_account;
        let new_authority_info = ctx.accounts.new_authority.key();

        data_info.authority = new_authority_info;
        Ok(())
    }

    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
        msg!("RecordInstruction::CloseAccount");
        let data_info = &mut ctx.accounts.record_account;
        let destination_info = &mut ctx.accounts.reciever;

        let destination_starting_lamports = destination_info.lamports();
        let data_lamports = data_info.to_account_info().lamports();
        
        **data_info.to_account_info().lamports.borrow_mut() = 0;
        **destination_info.to_account_info().lamports.borrow_mut() = destination_starting_lamports
            .checked_add(data_lamports)
            .ok_or(ProgramError::Custom(0))?;
        data_info.data = RecordData::default().data;
        Ok(())
    }

    pub fn write(ctx: Context<Write>, offset: u64, data: Vec<u8>) -> Result<()> {
        msg!("RecordInstruction::Write");
        let data_info = &mut ctx.accounts.record_account;
        let start = WRITABLE_START_INDEX + offset as usize;
        let end = start + data.len();
        if end > data_info.data.len() {
            return Err(ProgramError::AccountDataTooSmall.into());
        } else {
            data_info.data[start..end].copy_from_slice(&data);
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 1 + 8 + 32 + 1)]
    pub record_account: Account<'info, RecordData>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct SetAuthority<'info> {
    #[account(mut, has_one = authority)]
    pub record_account: Account<'info, RecordData>,
    pub authority: Signer<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub new_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(mut, has_one = authority)]
    pub record_account: Account<'info, RecordData>,
    pub authority: Signer<'info>,
    #[account(mut)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub reciever: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Write<'info> {
    #[account(mut, has_one = authority)]
    pub record_account: Account<'info, RecordData>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>
}

const DATA_SIZE: usize = 8;
const CURRENT_VERSION: u8 = 1;
const WRITABLE_START_INDEX: usize = 0;

#[account]
#[derive(Default)]
pub struct RecordData {
    /// Struct version, allows for upgrades to the program
    pub version: u8,

    /// The account allowed to update the data
    pub authority: Pubkey,

    /// The data contained by the account, could be anything serializable
    pub data: [u8; DATA_SIZE],
}


