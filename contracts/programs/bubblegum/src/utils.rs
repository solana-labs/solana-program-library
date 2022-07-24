use {
    crate::error::BubblegumError,
    crate::state::metaplex_adapter::MetadataArgs,
    crate::ASSET_PREFIX,
    anchor_lang::{
        prelude::*, solana_program::program_memory::sol_memcmp,
        solana_program::pubkey::PUBKEY_BYTES,
    },
    gummyroll::Node,
};

/// Assert that the provided MetadataArgs are compatible with MPL `Data`
pub fn assert_metadata_is_mpl_compatible(metadata: &MetadataArgs) -> Result<()> {
    if metadata.name.len() > mpl_token_metadata::state::MAX_NAME_LENGTH {
        return Err(BubblegumError::MetadataNameTooLong.into());
    }

    if metadata.symbol.len() > mpl_token_metadata::state::MAX_SYMBOL_LENGTH {
        return Err(BubblegumError::MetadataSymbolTooLong.into());
    }

    if metadata.uri.len() > mpl_token_metadata::state::MAX_URI_LENGTH {
        return Err(BubblegumError::MetadataUriTooLong.into());
    }

    if metadata.seller_fee_basis_points > 10000 {
        return Err(BubblegumError::MetadataBasisPointsTooHigh.into());
    }
    if metadata.creators.len() > 0 {
        if metadata.creators.len() > mpl_token_metadata::state::MAX_CREATOR_LIMIT - 1 {
            return Err(BubblegumError::CreatorsTooLong.into());
        }

        let mut total: u8 = 0;
        for i in 0..metadata.creators.len() {
            let creator = metadata.creators[i];
            for iter in metadata.creators.iter().skip(i + 1) {
                if iter.address == creator.address {
                    return Err(BubblegumError::DuplicateCreatorAddress.into());
                }
            }
            total = total
                .checked_add(creator.share)
                .ok_or(BubblegumError::CreatorShareTotalMustBe100)?;
        }
        if total != 100 {
            return Err(BubblegumError::CreatorShareTotalMustBe100.into());
        }
    }
    Ok(())
}

pub fn replace_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    gummyroll_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    merkle_roll: &AccountInfo<'info>,
    candy_wrapper: &AccountInfo<'info>,
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
            candy_wrapper: candy_wrapper.clone(),
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
    _append_authority: &AccountInfo<'info>,
    merkle_roll: &AccountInfo<'info>,
    candy_wrapper: &AccountInfo<'info>,
    leaf_node: Node,
) -> Result<()> {
    let seeds = &[seed.as_ref(), &[bump]];
    let authority_pda_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        gummyroll_program.clone(),
        gummyroll::cpi::accounts::Modify {
            authority: authority.clone(),
            merkle_roll: merkle_roll.clone(),
            candy_wrapper: candy_wrapper.clone(),
        },
        authority_pda_signer,
    );
    gummyroll::cpi::append(cpi_ctx, leaf_node)
}

pub fn cmp_pubkeys(a: &Pubkey, b: &Pubkey) -> bool {
    sol_memcmp(a.as_ref(), b.as_ref(), PUBKEY_BYTES) == 0
}

pub fn cmp_bytes(a: &[u8], b: &[u8], size: usize) -> bool {
    sol_memcmp(a.as_ref(), b.as_ref(), size) == 0
}

pub fn assert_pubkey_equal(
    a: &Pubkey,
    b: &Pubkey,
    error: Option<anchor_lang::error::Error>,
) -> Result<()> {
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

pub fn get_asset_id(tree_id: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            ASSET_PREFIX.as_ref(),
            tree_id.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &crate::id(),
    )
    .0
}
