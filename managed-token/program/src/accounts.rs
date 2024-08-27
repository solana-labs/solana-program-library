use {
    crate::assert_with_msg,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        program_error::ProgramError,
        system_program,
    },
};

pub struct InitializeMint<'a, 'info> {
    pub mint: &'a AccountInfo<'info>,
    pub payer: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> InitializeMint<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            mint: next_account_info(account_iter)?,
            payer: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            system_program: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.data_is_empty(),
            ProgramError::InvalidAccountData,
            "Mint account must be uninitialized",
        )?;
        assert_with_msg(
            ctx.mint.owner == &system_program::id(),
            ProgramError::IllegalOwner,
            "Mint account must be owned by the System Program when uninitialized",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.system_program.key == &system_program::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for System Program",
        )?;
        assert_with_msg(
            ctx.mint.is_writable,
            ProgramError::InvalidInstructionData,
            "Mint account must be writable",
        )?;
        assert_with_msg(
            ctx.payer.is_writable,
            ProgramError::InvalidInstructionData,
            "Payer account must be writable (lamport balance will change)",
        )?;
        assert_with_msg(
            ctx.payer.is_signer,
            ProgramError::MissingRequiredSignature,
            "Payer must sign for initialization",
        )?;
        Ok(ctx)
    }
}

pub struct InitializeAccount<'a, 'info> {
    pub token_account: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub payer: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub freeze_authority: &'a AccountInfo<'info>,
    pub mint: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub associated_token_program: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> InitializeAccount<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            token_account: next_account_info(account_iter)?,
            owner: next_account_info(account_iter)?,
            payer: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            freeze_authority: next_account_info(account_iter)?,
            mint: next_account_info(account_iter)?,
            system_program: next_account_info(account_iter)?,
            associated_token_program: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.token_account.data_is_empty(),
            ProgramError::InvalidAccountData,
            "Token account must be uninitialized",
        )?;
        assert_with_msg(
            ctx.token_account.owner == &system_program::id(),
            ProgramError::IllegalOwner,
            "Token account must be owned by System Program when uninitialized",
        )?;
        assert_with_msg(
            ctx.mint.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.system_program.key == &system_program::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for System Program",
        )?;
        assert_with_msg(
            ctx.associated_token_program.key == &spl_associated_token_account_client::program::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Associataed Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Token account must be writable",
        )?;
        assert_with_msg(
            ctx.payer.is_writable,
            ProgramError::InvalidInstructionData,
            "Payer account must be writable (lamport balance will change)",
        )?;
        assert_with_msg(
            ctx.payer.is_signer,
            ProgramError::MissingRequiredSignature,
            "Payer must sign for initialization",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for initialization",
        )?;
        Ok(ctx)
    }
}

pub struct Mint<'a, 'info> {
    pub mint: &'a AccountInfo<'info>,
    pub token_account: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub freeze_and_mint_authority: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> Mint<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            mint: next_account_info(account_iter)?,
            token_account: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            freeze_and_mint_authority: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.mint.is_writable,
            ProgramError::InvalidInstructionData,
            "Mint must be writable",
        )?;
        assert_with_msg(
            ctx.token_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Token Account must be writable",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for modification",
        )?;
        Ok(ctx)
    }
}

pub struct Burn<'a, 'info> {
    pub mint: &'a AccountInfo<'info>,
    pub token_account: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub freeze_authority: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> Burn<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            mint: next_account_info(account_iter)?,
            token_account: next_account_info(account_iter)?,
            owner: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            freeze_authority: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.mint.is_writable,
            ProgramError::InvalidInstructionData,
            "Mint must be writable",
        )?;
        assert_with_msg(
            ctx.token_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Token Account must be writable",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for modification",
        )?;
        assert_with_msg(
            ctx.owner.is_signer,
            ProgramError::MissingRequiredSignature,
            "Owner must sign for modification",
        )?;
        Ok(ctx)
    }
}

pub struct Transfer<'a, 'info> {
    pub src_account: &'a AccountInfo<'info>,
    pub dst_account: &'a AccountInfo<'info>,
    pub mint: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub freeze_authority: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> Transfer<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            src_account: next_account_info(account_iter)?,
            dst_account: next_account_info(account_iter)?,
            mint: next_account_info(account_iter)?,
            owner: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            freeze_authority: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.owner == &spl_token::id(),
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.src_account.owner == &spl_token::id(),
            ProgramError::IllegalOwner,
            "Source token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.dst_account.owner == &spl_token::id(),
            ProgramError::IllegalOwner,
            "Destination token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.src_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Source token account must be writable",
        )?;
        assert_with_msg(
            ctx.dst_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Destination token account must be writable",
        )?;
        assert_with_msg(
            ctx.owner.is_signer,
            ProgramError::MissingRequiredSignature,
            "Owner must sign for modification",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for modification",
        )?;
        Ok(ctx)
    }
}

pub struct Close<'a, 'info> {
    pub token_account: &'a AccountInfo<'info>,
    pub dst_account: &'a AccountInfo<'info>,
    pub mint: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub freeze_authority: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> Close<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            token_account: next_account_info(account_iter)?,
            dst_account: next_account_info(account_iter)?,
            mint: next_account_info(account_iter)?,
            owner: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            freeze_authority: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.dst_account.owner == &system_program::id(),
            ProgramError::IllegalOwner,
            "Destination account must be owned by the System Program",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Token Account must be writable",
        )?;
        assert_with_msg(
            ctx.dst_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Destination account must be writable",
        )?;
        assert_with_msg(
            ctx.owner.is_signer,
            ProgramError::MissingRequiredSignature,
            "Owner must sign for close",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for close",
        )?;
        Ok(ctx)
    }
}

pub struct Approve<'a, 'info> {
    pub mint: &'a AccountInfo<'info>,
    pub token_account: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub delegate: &'a AccountInfo<'info>,
    pub freeze_authority: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> Approve<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            mint: next_account_info(account_iter)?,
            token_account: next_account_info(account_iter)?,
            owner: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            delegate: next_account_info(account_iter)?,
            freeze_authority: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.owner == &spl_token::id(),
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Token Account must be writable",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for modification",
        )?;
        assert_with_msg(
            ctx.owner.is_signer,
            ProgramError::MissingRequiredSignature,
            "Owner must sign for modification",
        )?;
        Ok(ctx)
    }
}

pub struct Revoke<'a, 'info> {
    pub mint: &'a AccountInfo<'info>,
    pub token_account: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub upstream_authority: &'a AccountInfo<'info>,
    pub freeze_authority: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> Revoke<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let account_iter = &mut accounts.iter();
        let ctx = Self {
            mint: next_account_info(account_iter)?,
            token_account: next_account_info(account_iter)?,
            owner: next_account_info(account_iter)?,
            upstream_authority: next_account_info(account_iter)?,
            freeze_authority: next_account_info(account_iter)?,
            token_program: next_account_info(account_iter)?,
        };
        assert_with_msg(
            ctx.mint.owner == &spl_token::id(),
            ProgramError::IllegalOwner,
            "Mint account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.owner == ctx.token_program.key,
            ProgramError::IllegalOwner,
            "Token account must be owned by the Token Program",
        )?;
        assert_with_msg(
            ctx.token_account.is_writable,
            ProgramError::InvalidInstructionData,
            "Token Account must be writable",
        )?;
        assert_with_msg(
            ctx.token_program.key == &spl_token::id(),
            ProgramError::InvalidInstructionData,
            "Invalid key supplied for Token Program",
        )?;
        assert_with_msg(
            ctx.upstream_authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Freeze authority must sign for modification",
        )?;
        assert_with_msg(
            ctx.owner.is_signer,
            ProgramError::MissingRequiredSignature,
            "Owner must sign for modification",
        )?;
        Ok(ctx)
    }
}
