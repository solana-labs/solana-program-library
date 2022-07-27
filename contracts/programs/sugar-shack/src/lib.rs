use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        keccak::hashv,
        log::sol_log_compute_units,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
        system_instruction,
    },
};
use bubblegum::program::Bubblegum;
use gummyroll::program::Gummyroll;
use gummyroll::state::CandyWrapper;
use solana_safe_math::SafeMath;
pub mod state;
use crate::state::{MarketplaceProperties, MARKETPLACE_PROPERTIES_SIZE};

declare_id!("9T5Xv2cJRydUBqvdK7rLGuNGqhkA8sU8Yq1rGN7hExNK");

const MARKETPLACE_PROPERTIES_PREFIX: &str = "mymarketplace";

#[derive(Accounts)]
pub struct InitMarketplaceProperties<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = MARKETPLACE_PROPERTIES_SIZE,
        seeds = [MARKETPLACE_PROPERTIES_PREFIX.as_ref()],
        bump,
    )]
    pub marketplace_props: Account<'info, MarketplaceProperties>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMarketplaceProperties<'info> {
    /// CHECK: the authority over the marketplace, must correspond to the authority in marketplace_props, validated in instruction.
    pub authority: Signer<'info>,

    /// CHECK: the PDA for this marketplace.
    #[account(
        seeds = [MARKETPLACE_PROPERTIES_PREFIX.as_ref()],
        bump = marketplace_props.bump,
    )]
    #[account(mut)]
    pub marketplace_props: Account<'info, MarketplaceProperties>,
}

#[derive(Accounts)]
#[instruction(
    price: u64,
)]
pub struct CreateModifyListing<'info> {
    /// CHECK: should own NFT, validated downstream in Gummyroll
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: should be the current delegate of the NFT. Validated downstream in Gummyroll.
    pub former_delegate: AccountInfo<'info>,

    #[account(
        seeds = [price.to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: A PDA that encodes the price of the listing, will become the new delegate for the NFT.
    pub new_delegate: AccountInfo<'info>,

    /// CHECK: PDA is checked in CPI from Bubblegum to Gummyroll
    /// This key must sign for all write operations to the NFT Metadata stored in the Merkle slab
    pub bubblegum_authority: AccountInfo<'info>,
    pub gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Validation occurs in Gummyroll
    #[account(mut)]
    pub merkle_slab: AccountInfo<'info>,
    pub bubblegum: Program<'info, Bubblegum>,
    pub candy_wrapper: Program<'info, CandyWrapper>,
}

#[derive(Accounts)]
pub struct RemoveListing<'info> {
    /// CHECK: should own NFT, validated downstream in Gummyroll
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: should be the delegate for the NFT to be listed. Validated downstream in Gummyroll
    pub former_delegate: AccountInfo<'info>,

    /// CHECK: any delegate desired by the owner
    pub new_delegate: AccountInfo<'info>,

    /// CHECK: PDA is checked in CPI from Bubblegum to Gummyroll
    /// This key must sign for all write operations to the NFT Metadata stored in the Merkle slab
    pub bubblegum_authority: AccountInfo<'info>,
    pub gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Validation occurs in Gummyroll
    #[account(mut)]
    pub merkle_slab: AccountInfo<'info>,
    pub bubblegum: Program<'info, Bubblegum>,
    pub candy_wrapper: Program<'info, CandyWrapper>,
}

#[derive(Accounts)]
#[instruction(
    price: u64,
)]
pub struct Purchase<'info> {
    /// CHECK: should be the owner of an NFT with a current listing, validated downstream in Gummyroll
    #[account(mut)]
    pub former_owner: AccountInfo<'info>,

    /// CHECK: the purchaser of the NFT up for listing
    #[account(mut)]
    pub purchaser: Signer<'info>,

    /// CHECK: should be the delegate of the listed NFT. Must be a PDA owned by this program for operation to work. Validated downstream in Gummyroll.
    #[account(
        seeds = [price.to_le_bytes().as_ref()],
        bump,
    )]
    pub listing_delegate: AccountInfo<'info>,
    /// CHECK: PDA is checked in CPI from Bubblegum to Gummyroll
    /// This key must sign for all write operations to the NFT Metadata stored in the Merkle slab
    pub bubblegum_authority: AccountInfo<'info>,
    pub gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Validation occurs in Gummyroll
    #[account(mut)]
    pub merkle_slab: AccountInfo<'info>,
    pub bubblegum: Program<'info, Bubblegum>,
    /// CHECK: the PDA for this marketplace with it's fee information.
    #[account(
        seeds = [MARKETPLACE_PROPERTIES_PREFIX.as_ref()],
        bump = marketplace_props.bump,
    )]
    #[account(mut)]
    pub marketplace_props: Account<'info, MarketplaceProperties>,
    pub system_program: Program<'info, System>,
    pub candy_wrapper: Program<'info, CandyWrapper>,
}

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    /// CHECK: any pubkey the marketplace wants to withdraw fees to
    #[account(mut)]
    pub fee_payout_recipient: AccountInfo<'info>,

    /// CHECK: the authority over the marketplace, must correspond to the authority in marketplace_props, validated in instruction.
    pub authority: Signer<'info>,

    /// CHECK: the PDA for this marketplace to withdraw fees from.
    #[account(
        seeds = [MARKETPLACE_PROPERTIES_PREFIX.as_ref()],
        bump = marketplace_props.bump,
    )]
    #[account(mut)]
    pub marketplace_props: Account<'info, MarketplaceProperties>,
    pub system_program: Program<'info, System>,
    pub sysvar_rent: Sysvar<'info, Rent>,
}

// A helper function to CPI to Bubblegum delegate. Used for listing creation, modification and removal.
#[inline(always)]
fn modify_compressed_nft_delegate<'info>(
    owner: &Signer<'info>,
    former_delegate: &AccountInfo<'info>,
    new_delegate: &AccountInfo<'info>,
    bubblegum_authority: &AccountInfo<'info>,
    gummyroll: &Program<'info, Gummyroll>,
    merkle_slab: &AccountInfo<'info>,
    bubblegum: &Program<'info, Bubblegum>,
    candy_wrapper: &Program<'info, CandyWrapper>,
    remaining_accounts: &[AccountInfo<'info>],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
    root: [u8; 32],
) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        bubblegum.to_account_info(),
        bubblegum::cpi::accounts::Delegate {
            authority: bubblegum_authority.to_account_info(),
            owner: owner.to_account_info(),
            previous_delegate: former_delegate.to_account_info(),
            new_delegate: new_delegate.to_account_info(),
            gummyroll_program: gummyroll.to_account_info(),
            merkle_slab: merkle_slab.to_account_info(),
            candy_wrapper: candy_wrapper.to_account_info(),
        },
    )
    .with_remaining_accounts(remaining_accounts.to_vec());
    bubblegum::cpi::delegate(cpi_ctx, root, data_hash, creator_hash, nonce, index)?;
    Ok(())
}

#[program]
pub mod sugar_shack {
    use super::*;

    /// Initialize the singleton PDA that will store the marketplace's admin info, mainly related to royalties.
    pub fn initialize_marketplace(
        ctx: Context<InitMarketplaceProperties>,
        royalty_share: u16,
        authority: Pubkey,
    ) -> Result<()> {
        let marketplace_props_data = &mut ctx.accounts.marketplace_props;
        assert!(royalty_share <= 10000);
        marketplace_props_data.share = royalty_share;
        marketplace_props_data.authority = authority;
        marketplace_props_data.bump = *ctx.bumps.get("marketplace_props").unwrap();
        Ok(())
    }

    /// Enables the authority of the marketplace to update admin properties
    pub fn update_marketplace_properties<'info>(
        ctx: Context<UpdateMarketplaceProperties>,
        authority: Option<Pubkey>,
        share: Option<u16>,
    ) -> Result<()> {
        // This instruction must be signed by the authority to the marketplace
        assert_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.marketplace_props.authority
        );
        match authority {
            Some(ay) => ctx.accounts.marketplace_props.authority = ay,
            None => {}
        }
        match share {
            Some(s) => ctx.accounts.marketplace_props.share = s,
            None => {}
        }
        Ok(())
    }

    /// Enables the owner of a compressed NFT to list their NFT for sale, can also be used to modify the list price of an existing listing.
    pub fn create_or_modify_listing<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateModifyListing<'info>>,
        price: u64,
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        root: [u8; 32],
    ) -> Result<()> {
        modify_compressed_nft_delegate(
            &ctx.accounts.owner,
            &ctx.accounts.former_delegate,
            &ctx.accounts.new_delegate,
            &ctx.accounts.bubblegum_authority,
            &ctx.accounts.gummyroll,
            &ctx.accounts.merkle_slab,
            &ctx.accounts.bubblegum,
            &ctx.accounts.candy_wrapper,
            ctx.remaining_accounts,
            data_hash,
            creator_hash,
            nonce,
            index,
            root,
        )?;
        Ok(())
    }

    /// Enables the owner of a compressed NFT to remove their listing from the marketplace. The new_delegate specified in this instruction
    /// should not be a PDA owned by this program for removal to be effective.
    pub fn remove_listing<'info>(
        ctx: Context<'_, '_, '_, 'info, RemoveListing<'info>>,
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        root: [u8; 32],
    ) -> Result<()> {
        modify_compressed_nft_delegate(
            &ctx.accounts.owner,
            &ctx.accounts.former_delegate,
            &ctx.accounts.new_delegate,
            &ctx.accounts.bubblegum_authority,
            &ctx.accounts.gummyroll,
            &ctx.accounts.merkle_slab,
            &ctx.accounts.bubblegum,
            &ctx.accounts.candy_wrapper,
            ctx.remaining_accounts,
            data_hash,
            creator_hash,
            nonce,
            index,
            root,
        )?;
        Ok(())
    }

    /// Enables any user to purchase an NFT listed on the marketplace.
    /// @dev: To avoid overflow precision errors we generally avoid operations that would involve multiplying by f64s. (i.e. price * creator_share/100).
    ///       instead we compute the most smallest unit that could be paid out to an entity (a bip) and allocate bips via multiplication.
    /// @notice: The risk here is that certain creators or the marketplace itself might not receive their fee, if price * num_bips_for_entity < 10,000.
    /// @notice: Any fees not paid to creators/marketplace will be transferred to the lister.
    pub fn purchase<'info>(
        ctx: Context<'_, '_, '_, 'info, Purchase<'info>>,
        price: u64,
        metadata_args_hash: [u8; 32],
        nonce: u64,
        index: u32,
        root: [u8; 32],
        creator_shares: Vec<u8>,
        seller_fee_basis_points: u16,
    ) -> Result<()> {
        // The fees for the marketplace plus the seller_fee_basis points cannot exceed 100% of the price
        assert!(
            ctx.accounts
                .marketplace_props
                .share
                .safe_add(seller_fee_basis_points)?
                <= 10000
        );

        // First, payout the marketplace's royalty fee.
        let amount_to_pay_marketplace = (price as u128)
            .safe_mul(ctx.accounts.marketplace_props.share as u128)?
            .safe_div(10000)? as u64;
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.purchaser.key(),
                &ctx.accounts.marketplace_props.key(),
                amount_to_pay_marketplace,
            ),
            &[
                ctx.accounts.purchaser.to_account_info(),
                ctx.accounts.marketplace_props.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        let mut total_remaining_price_allocation = price.safe_sub(amount_to_pay_marketplace)?;

        // Second, payout each "creator". Creators are an immutable set of secondary marketplace sale royalty recipients.
        // Simultaneously, collect <address, share> pairs to prepare to compute creator_hash
        let total_creator_allocation = (price as u128)
            .safe_mul((seller_fee_basis_points as u128))?
            .safe_div(10000)? as u64;
        let mut amount_paid_out_to_creators = 0;
        let mut creator_data: Vec<Vec<u8>> = Vec::new();
        let (creator_accounts, proof_accounts) =
            ctx.remaining_accounts.split_at(creator_shares.len());
        let creator_accounts_iter = &mut creator_accounts.iter();
        for share in creator_shares.into_iter() {
            let current_creator_info = next_account_info(creator_accounts_iter)?;
            let amount_to_pay_creator = (total_creator_allocation as u128)
                .safe_mul((share as u128))?
                .safe_div(100)? as u64;
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.purchaser.key(),
                    &current_creator_info.key(),
                    amount_to_pay_creator,
                ),
                &[
                    ctx.accounts.purchaser.to_account_info(),
                    current_creator_info.clone(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
            amount_paid_out_to_creators =
                amount_paid_out_to_creators.safe_add(amount_to_pay_creator)?;
            creator_data.push([current_creator_info.key().as_ref(), &[share]].concat());
        }
        total_remaining_price_allocation =
            total_remaining_price_allocation.safe_sub(amount_paid_out_to_creators)?;

        // Third, we payout all remaining lamports from "price" which were not paid to the marketplace/creators to the lister
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.purchaser.key(),
                &ctx.accounts.former_owner.key(),
                total_remaining_price_allocation,
            ),
            &[
                ctx.accounts.purchaser.to_account_info(),
                ctx.accounts.former_owner.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Compute the creator hash using <address, share> pairs
        let creator_hash = hashv(
            creator_data
                .iter()
                .map(|c| c.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_ref(),
        );

        // CPI to Bubblegum to transfer the NFT to its new owner
        let price_seed = price.to_le_bytes();
        let seeds: &[&[u8]] = &[
            price_seed.as_ref(),
            &[*ctx.bumps.get("listing_delegate").unwrap()],
        ];
        let authority_pda_signer: &[&[&[u8]]] = &[&seeds[..]];

        // Get the data for the CPI
        let mut transfer_instruction_data = vec![163, 52, 200, 231, 140, 3, 69, 186];
        let data_hash =
            hashv(&[&metadata_args_hash, &seller_fee_basis_points.to_le_bytes()]).to_bytes();
        transfer_instruction_data.append(
            &mut bubblegum::instruction::Transfer {
                root,
                data_hash,
                creator_hash: creator_hash.to_bytes(),
                nonce,
                index,
            }
            .try_to_vec()?,
        );

        // Get the account metas for the CPI call
        // @notice: the reason why we need to manually call `to_account_metas` is because `Bubblegum::transfer` takes
        //          either the owner or the delegate as an optional signer. Since the delegate is a PDA in this case the
        //          client side code cannot set its is_signer flag to true, and Anchor drops it's is_signer flag when converting
        //          CpiContext to account metas on the CPI call since there is no Signer specified in the instructions context.
        // @TODO:   Consider TransferWithOwner and TransferWithDelegate instructions to avoid this slightly messy CPI
        let transfer_accounts = bubblegum::cpi::accounts::Transfer {
            authority: ctx.accounts.bubblegum_authority.to_account_info(),
            owner: ctx.accounts.former_owner.to_account_info(),
            delegate: ctx.accounts.listing_delegate.to_account_info(),
            new_owner: ctx.accounts.purchaser.to_account_info(),
            gummyroll_program: ctx.accounts.gummyroll.to_account_info(),
            merkle_slab: ctx.accounts.merkle_slab.to_account_info(),
            candy_wrapper: ctx.accounts.candy_wrapper.to_account_info(),
        };
        let mut transfer_account_metas = transfer_accounts.to_account_metas(None);
        for acct in transfer_account_metas.iter_mut() {
            if acct.pubkey == ctx.accounts.listing_delegate.key() {
                (*acct).is_signer = true;
            }
        }
        for node in proof_accounts.iter() {
            transfer_account_metas.push(AccountMeta::new_readonly(*node.key, false));
        }

        let mut transfer_cpi_account_infos = transfer_accounts.to_account_infos();
        transfer_cpi_account_infos.extend_from_slice(proof_accounts);
        invoke_signed(
            &Instruction {
                program_id: ctx.accounts.bubblegum.key(),
                accounts: transfer_account_metas,
                data: transfer_instruction_data,
            },
            &(transfer_cpi_account_infos[..]),
            authority_pda_signer,
        )?;
        Ok(())
    }

    /// Enables marketplace authority to withdraw some collected fees to an external account
    pub fn withdraw_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawFees<'info>>,
        lamports_to_withdraw: u64,
    ) -> Result<()> {
        // This instruction must be signed by the authority to the marketplace
        assert_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.marketplace_props.authority
        );
        let marketplace_props_balance_after_withdrawal = ctx
            .accounts
            .marketplace_props
            .to_account_info()
            .lamports()
            .safe_sub(lamports_to_withdraw)?;

        // The marketplace props account must be left with enough funds to be rent exempt after the withdrawal
        assert!(Rent::get()?.is_exempt(
            marketplace_props_balance_after_withdrawal,
            MARKETPLACE_PROPERTIES_SIZE
        ));

        // Transfer lamports from props PDA to fee_payout_recipient
        **ctx
            .accounts
            .marketplace_props
            .to_account_info()
            .try_borrow_mut_lamports()? = marketplace_props_balance_after_withdrawal;
        **ctx
            .accounts
            .fee_payout_recipient
            .try_borrow_mut_lamports()? = ctx
            .accounts
            .fee_payout_recipient
            .lamports()
            .safe_add(lamports_to_withdraw)?;
        Ok(())
    }
}
