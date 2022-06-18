use {
    crate::{NONCE_PREFIX},
    crate::error::BubblegumError,
    anchor_lang::{
        prelude::*,
        solana_program::program_memory::sol_memcmp,
        solana_program::pubkey::PUBKEY_BYTES,
    },
    gummyroll::Node
};


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
    gummyroll::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf, new_leaf, index)
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
    gummyroll::cpi::append(cpi_ctx, leaf_node)
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
    gummyroll::cpi::insert_or_append(cpi_ctx, root_node, leaf, index)
}

pub fn cmp_pubkeys(a: &Pubkey, b: &Pubkey) -> bool {
    sol_memcmp(a.as_ref(), b.as_ref(), PUBKEY_BYTES) == 0
}

pub fn assert_pubkey_equal(a: &Pubkey, b: &Pubkey, error: Option<anchor_lang::error::Error>) -> Result<()>  {
    if !cmp_pubkeys(a, b) {
        if error.is_some() {
            let err = error.unwrap();
            return Err(err);
        }
        return Err(BubblegumError::PublicKeyMismatch.into());
    } else {
        Ok(())
    }
}

pub fn assert_derivation(
    program_id: &Pubkey,
    account: &AccountInfo,
    path: &[&[u8]],
    error: Option<error::Error>,
) -> Result<u8> {
    let (key, bump) = Pubkey::find_program_address(&path, program_id);
    if !cmp_pubkeys(&key, account.key) {
        if error.is_some() {
            let err = error.unwrap();
            msg!("Derivation {:?}", err);
            return Err(err.into());
        }
        msg!("DerivedKeyInvalid");
        return Err(ProgramError::InvalidInstructionData.into());
    }
    Ok(bump)
}

pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> Result<()> {
    if !cmp_pubkeys(account.owner, owner) {
        //todo add better errors
        Err(ProgramError::IllegalOwner.into())
    } else {
        Ok(())
    }
}

pub fn get_asset_id(nonce_account: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            nonce_account.as_ref(),
            nonce.to_le_bytes().as_ref()
        ],
        &crate::id(),
    ).0
}

pub fn get_nonce_account_id(slab: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            NONCE_PREFIX.as_ref(),
            slab.as_ref()
        ],
        &crate::id(),
    )
}