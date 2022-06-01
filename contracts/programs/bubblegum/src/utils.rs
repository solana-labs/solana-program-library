use anchor_lang::prelude::*;
use gummyroll::Node;

pub fn replace_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    gummyroll_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    merkle_roll: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    root_node: Node,
    previous_leaf: Node,
    new_leaf: Node,
    index: u32,
) -> Result<()> {
    let seeds = &[seed.as_ref(), &[bump]];
    let authority_pda_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        gummyroll_program.clone(),
        gummyroll::cpi::accounts::Modify {
            authority: authority.clone(),
            merkle_roll: merkle_roll.clone(),
        },
        authority_pda_signer,
    )
    .with_remaining_accounts(remaining_accounts.to_vec());
    gummyroll::cpi::replace_leaf(cpi_ctx, *root_node, *previous_leaf, *new_leaf, index)
}

pub fn append_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    gummyroll_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    append_authority: &AccountInfo<'info>,
    merkle_roll: &AccountInfo<'info>,
    leaf_node: Node,
) -> Result<()> {
    let seeds = &[seed.as_ref(), &[bump]];
    let authority_pda_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        gummyroll_program.clone(),
        gummyroll::cpi::accounts::Append {
            authority: authority.clone(),
            append_authority: append_authority.clone(),
            merkle_roll: merkle_roll.clone(),
        },
        authority_pda_signer,
    );
    gummyroll::cpi::append(cpi_ctx, *leaf_node)
}

pub fn insert_or_append_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    gummyroll_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    merkle_roll: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    root_node: Node,
    leaf: Node,
    index: u32,
) -> Result<()> {
    let seeds = &[seed.as_ref(), &[bump]];
    let authority_pda_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        gummyroll_program.clone(),
        gummyroll::cpi::accounts::Modify {
            authority: authority.clone(),
            merkle_roll: merkle_roll.clone(),
        },
        authority_pda_signer,
    )
    .with_remaining_accounts(remaining_accounts.to_vec());
    gummyroll::cpi::insert_or_append(cpi_ctx, *root_node, *leaf, index)
}
