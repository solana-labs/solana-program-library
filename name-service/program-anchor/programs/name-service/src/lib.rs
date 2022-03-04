use std::convert::TryFrom;
use std::convert::TryInto;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_lang::AccountsClose;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod name_service {
    use super::*;

    pub fn create(
        ctx: Context<Create>,
        _hashed_name: Vec<u8>,
        _lamports: u64,
        space: u32,
    ) -> Result<()> {
        let name_account: &mut Account<NameRecord> = &mut ctx.accounts.account;
        let name_owner: &UncheckedAccount = &ctx.accounts.owner;
        let name_class: &UncheckedAccount = &ctx.accounts.class;
        let parent_name_account: &Account<NameRecord> = &ctx.accounts.parent_account;

        let name_state = NameRecord {
            parent_name: parent_name_account.key(),
            owner: *name_owner.key,
            class: *name_class.key,
            buffer: vec![0; space.try_into().unwrap()],
        };

        name_account.set_inner(name_state);

        Ok(())
    }

    pub fn update(ctx: Context<Update>, offset: u32, data: Vec<u8>) -> Result<()> {
        let account: &mut Account<NameRecord> = &mut ctx.accounts.account;

        let remaining_accounts_iter = &mut ctx.remaining_accounts.iter();

        let parent_name = next_account_info(remaining_accounts_iter).ok();

        let is_parent_owner = if let Some(parent_name) = parent_name {
            require!(
                account.parent_name == *parent_name.key,
                NameServiceError::InvalidParentAccount
            );
            true
        } else {
            false
        };

        require!(is_parent_owner, NameServiceError::InvalidParentAccount);

        let offset_size = usize::try_from(offset).unwrap();
        account.buffer[offset_size..offset_size + data.len()].copy_from_slice(&data);

        Ok(())
    }

    pub fn transfer(ctx: Context<Transfer>, new_owner: Pubkey) -> Result<()> {
        let account: &mut Account<NameRecord> = &mut ctx.accounts.account;
        let owner: &Signer = &ctx.accounts.owner;

        let remaining_accounts_iter = &mut ctx.remaining_accounts.iter();

        let class = next_account_info(remaining_accounts_iter).ok();
        let parent_unchecked_account = next_account_info(remaining_accounts_iter).ok();

        let is_parent_owner = if let Some(parent_name) = parent_unchecked_account {
            require!(
                account.parent_name != *parent_name.key,
                NameServiceError::InvalidParentAccount
            );
            let parent_data: &mut &[u8] = &mut &(*parent_name.try_borrow_mut_data().unwrap())[..];
            let parent_account = NameRecord::try_deserialize(parent_data)?;
            parent_account.owner == *owner.key
        } else {
            false
        };

        require!(
            owner.is_signer && (account.owner == *owner.key || !is_parent_owner),
            NameServiceError::OwnerNotSigner
        );

        require!(
            account.class == Pubkey::default()
                || (!class.is_none()
                    && account.class == *class.unwrap().key
                    && class.unwrap().is_signer),
            NameServiceError::ClassNotSigner
        );

        account.owner = new_owner;

        Ok(())
    }

    pub fn delete(ctx: Context<Delete>) -> Result<()> {
        let account: &mut Account<NameRecord> = &mut ctx.accounts.account;
        let refund_account: &UncheckedAccount = &ctx.accounts.refund_account;

        account.parent_name = Pubkey::default();
        account.owner = Pubkey::default();
        account.class = Pubkey::default();
        account.buffer = vec![0; account.buffer.len()];

        let _res = account.close(refund_account.to_account_info());

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(hashed_name: Vec<u8>,
    lamports: u64,
    space: u32,
)]
pub struct Create<'info> {
    #[account(constraint = parent_account.key() == Pubkey::default() || parent_owner.is_signer)]
    pub parent_owner: Signer<'info>,
    pub parent_account: Account<'info, NameRecord>,
    /// CHECK: We don't read or write to it
    #[account(constraint = class.key() == Pubkey::default() || class.is_signer)]
    pub class: UncheckedAccount<'info>,
    /// CHECK: We don't read or write to it
    #[account(constraint = owner.key() != Pubkey::default())]
    pub owner: UncheckedAccount<'info>,
    #[account(init_if_needed,
        payer = payer,
        space = 8 + NameRecord::LEN + usize::try_from(space).unwrap(),
        constraint = account.owner == Pubkey::default(),
        seeds = [hashed_name.as_ref()], bump
    )]
    pub account: Account<'info, NameRecord>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(constraint = (account.class == Pubkey::default() && owner.key() == account.owner.key()) ||
        (account.class != Pubkey::default() && owner.key() == account.class.key()))
    ]
    pub owner: Signer<'info>,
    #[account(mut, constraint = account.owner == Pubkey::default())]
    pub account: Account<'info, NameRecord>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    pub owner: Signer<'info>,
    #[account(mut)]
    pub account: Account<'info, NameRecord>,
}

#[derive(Accounts)]
pub struct Delete<'info> {
    /// CHECK: We don't read or write to it
    pub refund_account: UncheckedAccount<'info>,
    #[account()]
    pub owner: Signer<'info>,
    #[account(mut, constraint = account.owner == owner.key())]
    pub account: Account<'info, NameRecord>,
}

#[account]
#[derive(Default)]
pub struct NameRecord {
    pub parent_name: Pubkey,
    pub owner: Pubkey,
    pub class: Pubkey,
    pub buffer: Vec<u8>,
}

impl NameRecord {
    pub const LEN: usize = 32 + 32 + 32;
}

#[error]
pub enum NameServiceError {
    #[msg("Out of space")]
    OutOfSpace,
    #[msg("Invalid parent account")]
    InvalidParentAccount,
    #[msg("Owner is incorrect or not a signer")]
    OwnerNotSigner,
    #[msg("Class is incorrect or not a signer")]
    ClassNotSigner,
    #[msg("Account close error")]
    AccountClose,
}
