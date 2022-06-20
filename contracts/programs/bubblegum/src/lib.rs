use {
    crate::state::metaplex_anchor::MplTokenMetadata,
    crate::state::{
        NONCE_SIZE,
        VOUCHER_PREFIX,
        VOUCHER_SIZE,
        ASSET_PREFIX,
        leaf_schema::{LeafSchema, Version},
        metaplex_anchor::{MasterEdition, TokenMetadata},
        Nonce, Voucher,
        metaplex_adapter::{MetadataArgs, TokenProgramVersion},
        NewNFTEvent,
        NFTDecompressionEvent
    },
    gummyroll::{program::Gummyroll, Node},
    crate::error::BubblegumError,
    crate::utils::{append_leaf,
                   insert_or_append_leaf,
                   replace_leaf,
                   get_asset_id,
                   cmp_bytes,
                   cmp_pubkeys,
                   assert_pubkey_equal,
    },
    anchor_lang::{
        prelude::*,
        solana_program::{
            keccak,
            program::{invoke, invoke_signed},
            program_error::ProgramError,
            program_pack::Pack,
            system_instruction,
        },
    },
    spl_token::state::Mint as SplMint,
};


pub mod state;
pub mod utils;
pub mod error;

declare_id!("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY");

#[derive(Accounts)]
pub struct CreateTree<'info> {
    #[account(
    init,
    seeds = [merkle_slab.key().as_ref()],
    payer = payer,
    space = NONCE_SIZE,
    bump,
    )]
    pub authority: Account<'info, Nonce>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub tree_creator: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(zero)]
    /// CHECK: This account must be all zeros
    pub merkle_slab: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MintV1<'info> {
    /// CHECK: This account is neither written to nor read from.
    pub mint_authority: Signer<'info>,
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    #[account(mut)]
    pub authority: Account<'info, Nonce>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    /// CHECK: This account is neither written to nor read from.
    pub owner: AccountInfo<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub delegate: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_slab: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: Account<'info, Nonce>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    /// CHECK: This account is checked in the instruction
    pub owner: UncheckedAccount<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_slab: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: Account<'info, Nonce>,
    /// CHECK: This account is checked in the instruction
    pub owner: UncheckedAccount<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_owner: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_slab: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Delegate<'info> {
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: Account<'info, Nonce>,
    pub owner: Signer<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub previous_delegate: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_delegate: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_slab: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(
_root: [u8; 32],
_data_hash: [u8; 32],
_creator_hash: [u8; 32],
nonce: u64,
_index: u32,
)]
pub struct Redeem<'info> {
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: Account<'info, Nonce>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked in cpi
    pub merkle_slab: UncheckedAccount<'info>,
    #[account(
    init,
    seeds = [
    VOUCHER_PREFIX.as_ref(),
    merkle_slab.key().as_ref(),
    & nonce.to_le_bytes()
    ],
    payer = owner,
    space = VOUCHER_SIZE,
    bump
    )]
    pub voucher: Account<'info, Voucher>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelRedeem<'info> {
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: Account<'info, Nonce>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_slab: UncheckedAccount<'info>,
    #[account(
    mut,
    close = owner,
    seeds = [
    VOUCHER_PREFIX.as_ref(),
    merkle_slab.key().as_ref(),
    & voucher.leaf_schema.nonce().to_le_bytes()
    ],
    bump
    )]
    pub voucher: Account<'info, Voucher>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct DecompressV1<'info> {
    #[account(
    mut,
    close = owner,
    seeds = [
    VOUCHER_PREFIX.as_ref(),
    voucher.merkle_slab.as_ref(),
    voucher.leaf_schema.nonce().to_le_bytes().as_ref()
    ],
    bump
    )]
    pub voucher: Box<Account<'info, Voucher>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub token_account: UncheckedAccount<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(
    mut,
    seeds = [
    voucher.leaf_schema.id().as_ref(),
    token_program.key().as_ref()
    ],
    bump
    )]
    pub mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(
    seeds = [mint.key().as_ref()],
    bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: Initialized in Token Metadata Program
    #[account(mut)]
    pub master_edition: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub sysvar_rent: Sysvar<'info, Rent>,
    /// CHECK:
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
    /// CHECK: versioning is handled in the instruction
    pub token_program: UncheckedAccount<'info>,
    /// CHECK:
    pub associated_token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Compress<'info> {
    #[account(
    seeds = [merkle_slab.key().as_ref()],
    bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: This account is not read
    pub merkle_slab: UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub owner: Signer<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub metadata: Box<Account<'info, TokenMetadata>>,
    #[account(mut)]
    pub master_edition: Box<Account<'info, MasterEdition>>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
}

pub fn hash_metadata(metadata: &MetadataArgs) -> Result<[u8; 32]> {
    Ok(keccak::hashv(&[metadata.try_to_vec()?.as_slice()]).to_bytes())
}

pub enum InstructionName {
    Unknown,
    MintV1,
    Redeem,
    CancelRedeem,
    Transfer,
    Delegate,
    DecompressV1,
    Compress,
    Burn,
}

pub fn get_instruction_type(full_bytes: &[u8]) -> InstructionName {
    let disc: [u8; 8] = {
        let mut disc = [0; 8];
        disc.copy_from_slice(&full_bytes[..8]);
        disc
    };
    match disc {
        [145, 98, 192, 118, 184, 147, 118, 104] => InstructionName::MintV1,
        [111, 76, 232, 50, 39, 175, 48, 242] => InstructionName::CancelRedeem,
        [184, 12, 86, 149, 70, 196, 97, 225] => InstructionName::Redeem,
        [163, 52, 200, 231, 140, 3, 69, 186] => InstructionName::Transfer,
        [90, 147, 75, 178, 85, 88, 4, 137] => InstructionName::Delegate,
        [54, 85, 76, 70, 228, 250, 164, 81] => InstructionName::DecompressV1,
        [116, 110, 29, 56, 107, 219, 42, 93] => InstructionName::Burn,
        [82, 193, 176, 117, 176, 21, 115, 253] => InstructionName::Compress,
        _ => InstructionName::Unknown,
    }
}

#[program]
pub mod bubblegum {
    use super::*;

    pub fn create_tree(
        ctx: Context<CreateTree>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        let seed = merkle_slab.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("authority").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.gummyroll_program.to_account_info(),
            gummyroll::cpi::accounts::Initialize {
                authority: ctx.accounts.authority.to_account_info(),
                append_authority: ctx.accounts.tree_creator.to_account_info(),
                merkle_roll: merkle_slab,
            },
            authority_pda_signer,
        );
        gummyroll::cpi::init_empty_gummyroll(cpi_ctx, max_depth, max_buffer_size)
    }

    pub fn mint_v1(ctx: Context<MintV1>, message: MetadataArgs) -> Result<()> {
        // TODO -> Pass collection in check collection authority or collection delegate authority signer
        // TODO -> Separate V1 / V1 into seperate instructions
        //
        let owner = ctx.accounts.owner.key();
        let delegate = ctx.accounts.delegate.key();
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        let data_hash = keccak::hashv(&[message.try_to_vec()?.as_slice()]);
        let nonce = &mut ctx.accounts.authority;
        let creator_data = message
            .creators
            .iter()
            //TODO include verified
            .map(|c| [c.address.as_ref(), &[c.share]].concat())
            .collect::<Vec<_>>();
        let creator_hash = keccak::hashv(
            creator_data
                .iter()
                .map(|c| c.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_ref(),
        );
        let asset_id = get_asset_id(&merkle_slab.key(), nonce.count);
        let leaf = LeafSchema::new_v0(
            asset_id,
            owner,
            delegate,
            nonce.count,
            data_hash.to_bytes(),
            creator_hash.to_bytes(),
        );
        emit!(NewNFTEvent {
            version: Version::V1,
            metadata: message,
            nonce: nonce.count
        });
        emit!(leaf.to_event());
        nonce.count = nonce.count.saturating_add(1);
        append_leaf(
            &merkle_slab.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.merkle_slab.to_account_info(),
            leaf.to_node(),
        )
    }

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Transfer<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        // TODO add back version to select hash schema
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let delegate = ctx.accounts.delegate.to_account_info();
        // Transfers must be initiated either by the leaf owner or leaf delegate
        assert!(owner.is_signer || delegate.is_signer);
        let new_owner = ctx.accounts.new_owner.key();
        let asset_id = get_asset_id(&merkle_slab.key(), nonce);
        let previous_leaf = LeafSchema::new_v0(
            asset_id,
            owner.key(),
            delegate.key(),
            nonce,
            data_hash,
            creator_hash,
        );
        // New leafs are instantiated with no delegate
        let new_leaf = LeafSchema::new_v0(
            asset_id,
            new_owner,
            new_owner,
            nonce,
            data_hash,
            creator_hash,
        );
        emit!(new_leaf.to_event());
        replace_leaf(
            &merkle_slab.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_slab.to_account_info(),
            ctx.remaining_accounts,
            root,
            previous_leaf.to_node(),
            new_leaf.to_node(),
            index,
        )
    }

    pub fn delegate<'info>(
        ctx: Context<'_, '_, '_, 'info, Delegate<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        let owner = ctx.accounts.owner.key();
        let previous_delegate = ctx.accounts.previous_delegate.key();
        let new_delegate = ctx.accounts.new_delegate.key();
        let asset_id = get_asset_id(&merkle_slab.key(), nonce);
        let previous_leaf = LeafSchema::new_v0(
            asset_id,
            owner,
            previous_delegate,
            nonce,
            data_hash,
            creator_hash,
        );
        let new_leaf =
            LeafSchema::new_v0(asset_id, owner, new_delegate, nonce, data_hash, creator_hash);
        emit!(new_leaf.to_event());
        replace_leaf(
            &merkle_slab.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_slab.to_account_info(),
            ctx.remaining_accounts,
            root,
            previous_leaf.to_node(),
            new_leaf.to_node(),
            index,
        )
    }

    pub fn burn<'info>(
        ctx: Context<'_, '_, '_, 'info, Burn<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        let owner = ctx.accounts.owner.to_account_info();
        let delegate = ctx.accounts.delegate.to_account_info();
        assert!(owner.is_signer || delegate.is_signer);
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        let asset_id = get_asset_id(&merkle_slab.key(), nonce);
        let previous_leaf = LeafSchema::new_v0(
            asset_id,
            owner.key(),
            delegate.key(),
            nonce,
            data_hash,
            creator_hash,
        );
        emit!(previous_leaf.to_event());
        let new_leaf = Node::default();
        replace_leaf(
            &merkle_slab.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_slab.to_account_info(),
            ctx.remaining_accounts,
            root,
            previous_leaf.to_node(),
            new_leaf,
            index,
        )
    }

    pub fn redeem<'info>(
        ctx: Context<'_, '_, '_, 'info, Redeem<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        let owner = ctx.accounts.owner.key();
        let delegate = ctx.accounts.delegate.key();
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        let asset_id = get_asset_id(&merkle_slab.key(), nonce);
        let previous_leaf =
            LeafSchema::new_v0(asset_id, owner, delegate, nonce, data_hash, creator_hash);
        emit!(previous_leaf.to_event());
        let new_leaf = Node::default();
        replace_leaf(
            &merkle_slab.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_slab.to_account_info(),
            ctx.remaining_accounts,
            root,
            previous_leaf.to_node(),
            new_leaf,
            index,
        )?;
        ctx.accounts
            .voucher
            .set_inner(Voucher::new(previous_leaf, index, merkle_slab.key()));

        Ok(())
    }

    pub fn cancel_redeem<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelRedeem<'info>>,
        root: [u8; 32],
    ) -> Result<()> {
        let voucher = &ctx.accounts.voucher;
        match ctx.accounts.voucher.leaf_schema {
            LeafSchema::V1 { owner, .. } =>
                assert_pubkey_equal(&ctx.accounts.owner.key(),
                                    &owner,
                                    Some(BubblegumError::AssetOwnerMismatch.into()),
                ),
        }?;
        let merkle_slab = ctx.accounts.merkle_slab.to_account_info();
        emit!(voucher.leaf_schema.to_event());
        insert_or_append_leaf(
            &merkle_slab.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_slab.to_account_info(),
            ctx.remaining_accounts,
            root,
            voucher.leaf_schema.to_node(),
            voucher.index,
        )
    }

    pub fn decompress_v1(ctx: Context<DecompressV1>, metadata: MetadataArgs) -> Result<()> {
        // Allocate and create mint
        let incoming_data_hash = hash_metadata(&metadata)?;
        let event = match ctx.accounts.voucher.leaf_schema {
            LeafSchema::V1 {
                owner,
                data_hash,
                nonce,
                ..
            } => {
                if !cmp_bytes(&data_hash, &incoming_data_hash, 32) {
                    return Err(BubblegumError::HashingMismatch.into());
                }
                if !cmp_pubkeys(&owner, ctx.accounts.owner.key) {
                    return Err(BubblegumError::AssetOwnerMismatch.into());
                }
                Ok(NFTDecompressionEvent {
                    version: Version::V1,
                    tree_id: ctx.accounts.voucher.merkle_slab.key(),
                    id: get_asset_id(&ctx.accounts.voucher.merkle_slab.key(), nonce),
                    nonce: nonce
                })
            }
            _ => Err(BubblegumError::UnsupportedSchemaVersion)
        }?;
        let voucher = &ctx.accounts.voucher;
        match metadata.token_program_version {
            TokenProgramVersion::Original => {
                if ctx.accounts.mint.data_is_empty() {
                    invoke_signed(
                        &system_instruction::create_account(
                            &ctx.accounts.owner.key(),
                            &ctx.accounts.mint.key(),
                            Rent::get()?.minimum_balance(SplMint::LEN),
                            SplMint::LEN as u64,
                            &spl_token::id(),
                        ),
                        &[
                            ctx.accounts.owner.to_account_info(),
                            ctx.accounts.mint.to_account_info(),
                            ctx.accounts.system_program.to_account_info(),
                        ],
                        &[&[
                            voucher.leaf_schema.id().as_ref(),
                            spl_token::id().as_ref(),
                            &[*ctx.bumps.get("mint").unwrap()],
                        ]],
                    )?;
                    invoke(
                        &spl_token::instruction::initialize_mint2(
                            &spl_token::id(),
                            &ctx.accounts.mint.key(),
                            &ctx.accounts.mint_authority.key(),
                            None,
                            0,
                        )?,
                        &[
                            ctx.accounts.token_program.to_account_info(),
                            ctx.accounts.mint.to_account_info(),
                        ],
                    )?;
                }
                if ctx.accounts.token_account.data_is_empty() {
                    invoke(
                        &spl_associated_token_account::instruction::create_associated_token_account(
                            &ctx.accounts.owner.key(),
                            &ctx.accounts.owner.key(),
                            &ctx.accounts.mint.key(),
                        ),
                        &[
                            ctx.accounts.owner.to_account_info(),
                            ctx.accounts.mint.to_account_info(),
                            ctx.accounts.token_account.to_account_info(),
                            ctx.accounts.token_program.to_account_info(),
                            ctx.accounts.associated_token_program.to_account_info(),
                            ctx.accounts.system_program.to_account_info(),
                            ctx.accounts.sysvar_rent.to_account_info(),
                        ],
                    )?;
                }
                invoke_signed(
                    &spl_token::instruction::mint_to(
                        &spl_token::id(),
                        &ctx.accounts.mint.key(),
                        &ctx.accounts.token_account.key(),
                        &ctx.accounts.mint_authority.key(),
                        &[],
                        1,
                    )?,
                    &[
                        ctx.accounts.mint.to_account_info(),
                        ctx.accounts.token_account.to_account_info(),
                        ctx.accounts.mint_authority.to_account_info(),
                        ctx.accounts.token_program.to_account_info(),
                    ],
                    &[&[
                        ctx.accounts.mint.key().as_ref(),
                        &[ctx.bumps["mint_authority"]],
                    ]],
                )?;
            }
            TokenProgramVersion::Token2022 => return Err(ProgramError::InvalidArgument.into()),
        }

        let metadata_infos = vec![
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.sysvar_rent.to_account_info(),
        ];

        let master_edition_infos = vec![
            ctx.accounts.master_edition.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.sysvar_rent.to_account_info(),
        ];

        msg!("Creating metadata!");
        invoke_signed(
            &mpl_token_metadata::instruction::create_metadata_accounts_v2(
                ctx.accounts.token_metadata_program.key(),
                ctx.accounts.metadata.key(),
                ctx.accounts.mint.key(),
                ctx.accounts.mint_authority.key(),
                ctx.accounts.owner.key(),
                ctx.accounts.mint_authority.key(),
                metadata.name.clone(),
                metadata.symbol.clone(),
                metadata.uri.clone(),
                if metadata.creators.len() > 0 {
                    Some(metadata.creators.iter().map(|c| c.adapt()).collect())
                } else {
                    None
                },
                metadata.seller_fee_basis_points,
                true,
                metadata.is_mutable,
                match metadata.collection {
                    Some(c) => Some(c.adapt()),
                    None => None,
                },
                match metadata.uses {
                    Some(u) => Some(u.adapt()),
                    None => None,
                },
            ),
            metadata_infos.as_slice(),
            &[&[
                ctx.accounts.mint.key().as_ref(),
                &[ctx.bumps["mint_authority"]],
            ]],
        )?;

        msg!("Creating master edition!");
        invoke_signed(
            &mpl_token_metadata::instruction::create_master_edition_v3(
                ctx.accounts.token_metadata_program.key(),
                ctx.accounts.master_edition.key(),
                ctx.accounts.mint.key(),
                ctx.accounts.mint_authority.key(),
                ctx.accounts.mint_authority.key(),
                ctx.accounts.metadata.key(),
                ctx.accounts.owner.key(),
                Some(0),
            ),
            master_edition_infos.as_slice(),
            &[&[
                ctx.accounts.mint.key().as_ref(),
                &[ctx.bumps["mint_authority"]],
            ]],
        )?;
        emit!(event);
        Ok(())
    }

    pub fn compress(_ctx: Context<Compress>) -> Result<()> {
        // TODO
        Ok(())
    }
}
