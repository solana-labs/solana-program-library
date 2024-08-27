use {
    crate::get_authority,
    borsh::{BorshDeserialize, BorshSerialize},
    shank::ShankInstruction,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
    spl_associated_token_account_client::address::get_associated_token_address,
};

#[derive(Debug, Clone, ShankInstruction, BorshSerialize, BorshDeserialize)]
#[rustfmt::skip]
pub enum ManagedTokenInstruction {

    #[account(0, writable, signer, name = "mint")]
    #[account(1, writable, signer, name = "payer")]
    #[account(2, name = "upstream_authority")]
    #[account(3, name = "system_program", desc = "System program")]
    #[account(4, name = "token_program", desc = "Token program")]
    InitializeMint {
        decimals: u8,
    },

    #[account(0, writable, name = "account")]
    #[account(1, name = "owner")]
    #[account(2, writable, signer, name = "payer")]
    #[account(3, signer, name = "upstream_authority")]
    #[account(4, name = "freeze_authority")]
    #[account(5, name = "mint")]
    #[account(6, name = "system_program", desc = "System program")]
    #[account(
        7,
        name = "associated_token_program",
        desc = "Associated Token program"
    )]
    #[account(8, name = "token_program", desc = "Token program")]
    InitializeAccount,

    #[account(0, writable, name = "src_account")]
    #[account(1, writable, name = "dst_account")]
    #[account(2, name = "mint")]
    #[account(3, signer, name = "owner")]
    #[account(4, signer, name = "upstream_authority")]
    #[account(5, name = "freeze_authority")]
    #[account(6, name = "token_program", desc = "Token program")]
    Transfer { amount: u64 },

    #[account(0, writable, name = "mint")]
    #[account(1, writable, name = "account")]
    #[account(2, signer, name = "upstream_authority")]
    #[account(3, name = "freeze_authority")]
    #[account(4, name = "token_program", desc = "Token program")]
    MintTo { amount: u64 },

    #[account(0, writable, name = "mint")]
    #[account(1, writable, name = "account")]
    #[account(2, signer, name = "owner")]
    #[account(3, signer, name = "upstream_authority")]
    #[account(4, name = "freeze_authority")]
    #[account(5, name = "token_program", desc = "Token program")]
    Burn { amount: u64 },

    #[account(0, writable, name = "account")]
    #[account(1, writable, name = "destination")]
    #[account(2, name = "mint")]
    #[account(3, signer, name = "owner")]
    #[account(4, signer, name = "upstream_authority")]
    #[account(5, name = "freeze_authority")]
    #[account(6, name = "token_program", desc = "Token program")]
    CloseAccount,

    #[account(0, name = "mint")]
    #[account(1, writable, name = "account")]
    #[account(2, signer, name = "owner")]
    #[account(3, signer, name = "upstream_authority")]
    #[account(4, name = "delegate")]
    #[account(5, name = "freeze_authority")]
    #[account(6, name = "token_program", desc = "Token program")]
    Approve { amount: u64 },

    #[account(0, name = "mint")]
    #[account(1, writable, name = "account")]
    #[account(2, signer, name = "owner")]
    #[account(3, signer, name = "upstream_authority")]
    #[account(4, name = "freeze_authority")]
    #[account(5, name = "token_program", desc = "Token program")]
    Revoke,
}

pub fn create_initialize_mint_instruction(
    mint: &Pubkey,
    payer: &Pubkey,
    upstream_authority: &Pubkey,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(*mint, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*upstream_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::InitializeMint { decimals })?,
    })
}

pub fn create_initialize_account_instruction(
    mint: &Pubkey,
    owner: &Pubkey,
    payer: &Pubkey,
    upstream_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let account = get_associated_token_address(owner, mint);
    let (freeze_authority, _) = get_authority(upstream_authority);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account_client::program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::InitializeAccount)?,
    })
}

pub fn create_mint_to_instruction(
    mint: &Pubkey,
    owner: &Pubkey,
    upstream_authority: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let account = get_associated_token_address(owner, mint);
    let (authority, _) = get_authority(upstream_authority);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::MintTo { amount })?,
    })
}

pub fn create_transfer_instruction(
    src: &Pubkey,
    dst: &Pubkey,
    mint: &Pubkey,
    upstream_authority: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let src_account = get_associated_token_address(src, mint);
    let dst_account = get_associated_token_address(dst, mint);
    let (freeze_authority, _) = get_authority(upstream_authority);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(src_account, false),
            AccountMeta::new(dst_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*src, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::Transfer { amount })?,
    })
}

pub fn create_transfer_with_delegate_instruction(
    src: &Pubkey,
    dst: &Pubkey,
    delegate: &Pubkey,
    mint: &Pubkey,
    upstream_authority: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let src_account = get_associated_token_address(src, mint);
    let dst_account = get_associated_token_address(dst, mint);
    let (freeze_authority, _) = get_authority(upstream_authority);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(src_account, false),
            AccountMeta::new(dst_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*delegate, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::Transfer { amount })?,
    })
}

pub fn create_burn_instruction(
    mint: &Pubkey,
    owner: &Pubkey,
    upstream_authority: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let account = get_associated_token_address(owner, mint);
    let (freeze_authority, _) = get_authority(upstream_authority);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::Burn { amount })?,
    })
}

pub fn create_close_account_instruction(
    mint: &Pubkey,
    owner: &Pubkey,
    upstream_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let account = get_associated_token_address(owner, mint);
    let (freeze_authority, _) = get_authority(upstream_authority);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(account, false),
            AccountMeta::new(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::CloseAccount)?,
    })
}

pub fn create_approve_instruction(
    mint: &Pubkey,
    owner: &Pubkey,
    delegate: &Pubkey,
    upstream_authority: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (freeze_authority, _) = get_authority(upstream_authority);
    let account = get_associated_token_address(owner, mint);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(*delegate, false),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::Approve { amount })?,
    })
}

pub fn create_revoke_instruction(
    mint: &Pubkey,
    owner: &Pubkey,
    upstream_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (freeze_authority, _) = get_authority(upstream_authority);
    let account = get_associated_token_address(owner, mint);
    Ok(Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*upstream_authority, true),
            AccountMeta::new_readonly(freeze_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: borsh::to_vec(&ManagedTokenInstruction::Revoke)?,
    })
}
