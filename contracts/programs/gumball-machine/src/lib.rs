use anchor_lang::{
    prelude::*,
    solana_program::{
        keccak::hashv, program::invoke, pubkey::Pubkey, system_instruction, sysvar,
        sysvar::instructions::load_instruction_at_checked, sysvar::SysvarId,
    },
};
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};
use bubblegum::program::Bubblegum;

use bubblegum::state::metaplex_adapter::MetadataArgs;
use bytemuck::cast_slice_mut;
use gummyroll::{program::Gummyroll, state::CandyWrapper};
use spl_token::native_mint;

pub mod state;
use crate::state::{GumballCreatorAdapter, NUM_CREATORS};

pub mod utils;

use crate::state::{EncodeMethod, GumballMachineHeader, ZeroCopy};
use crate::utils::get_metadata_args;

declare_id!("GBALLoMcmimUutWvtNdFFGH5oguS7ghUUV6toQPppuTW");

const COMPUTE_BUDGET_ADDRESS: &str = "ComputeBudget111111111111111111111111111111";
const MAX_NUM_INDICES_TO_INIT_FOR_CHUNK: u32 = 250000;

#[derive(Accounts)]
pub struct InitGumballMachine<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(zero)]
    gumball_machine: AccountInfo<'info>,
    #[account(mut)]
    payer: Signer<'info>,
    mint: Account<'info, Mint>,
    /// CHECK: Mint/append authority to the merkle slab
    #[account(
        seeds = [gumball_machine.key().as_ref()],
        bump,
    )]
    willy_wonka: AccountInfo<'info>,
    /// CHECK: Tree authority to the merkle slab, PDA owned by BubbleGum
    #[account(mut)]
    bubblegum_authority: AccountInfo<'info>,
    candy_wrapper: Program<'info, CandyWrapper>,
    gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Empty merkle slab
    #[account(zero)]
    merkle_slab: AccountInfo<'info>,
    bubblegum: Program<'info, Bubblegum>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitIndices<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateConfigLine<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateHeaderMetadata<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DispenseSol<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,

    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    receiver: AccountInfo<'info>,
    system_program: Program<'info, System>,

    #[account(
        seeds = [gumball_machine.key().as_ref()],
        bump,
    )]
    /// CHECK: PDA is checked on CPI for mint
    willy_wonka: AccountInfo<'info>,
    /// CHECK: Address is verified
    #[account(address = SlotHashes::id())]
    recent_blockhashes: UncheckedAccount<'info>,
    /// CHECK: Address is verified
    #[account(address = sysvar::instructions::id())]
    instruction_sysvar_account: AccountInfo<'info>,
    /// CHECK: PDA is checked in CPI from Bubblegum to Gummyroll
    /// This key must sign for all write operations to the NFT Metadata stored in the Merkle slab
    #[account(mut)]
    bubblegum_authority: AccountInfo<'info>,
    candy_wrapper: Program<'info, CandyWrapper>,
    gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Validation occurs in Gummyroll
    #[account(mut)]
    merkle_slab: AccountInfo<'info>,
    bubblegum: Program<'info, Bubblegum>,
}

#[derive(Accounts)]
pub struct DispenseToken<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,

    payer: Signer<'info>,

    #[account(mut)]
    payer_tokens: Account<'info, TokenAccount>,

    #[account(mut)]
    receiver: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,

    #[account(
        seeds = [gumball_machine.key().as_ref()],
        bump,
    )]
    /// CHECK: PDA is checked on CPI for mint
    willy_wonka: AccountInfo<'info>,
    /// CHECK: Address is verified
    #[account(address = SlotHashes::id())]
    recent_blockhashes: UncheckedAccount<'info>,
    /// CHECK: Address is verified
    #[account(address = sysvar::instructions::id())]
    instruction_sysvar_account: AccountInfo<'info>,
    /// CHECK: PDA is checked in CPI from Bubblegum to Gummyroll
    /// This key must sign for all write operations to the NFT Metadata stored in the Merkle slab
    #[account(mut)]
    bubblegum_authority: AccountInfo<'info>,
    candy_wrapper: Program<'info, CandyWrapper>,
    gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Validation occurs in Gummyroll
    #[account(mut)]
    merkle_slab: AccountInfo<'info>,
    bubblegum: Program<'info, Bubblegum>,
}

#[derive(Accounts)]
pub struct Destroy<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    #[account(mut)]
    authority: Signer<'info>,
}

#[inline(always)]
// Bots may try to buy only valuable NFTs by sending instructions to dispense an NFT along with
// instructions that fail if they do not get the one that they want. We prevent this by forcing
// all transactions that hit the "dispense" functions to have a single instruction body, and
// that the call to "dispense" is the top level of the single instruction (not a CPI)
fn assert_valid_single_instruction_transaction<'info>(
    instruction_sysvar_account: &AccountInfo<'info>,
) -> Result<()> {
    // There should only be one non compute-budget instruction
    // in this transaction (i.e. the current call to dispense_...)
    let instruction_sysvar = instruction_sysvar_account.try_borrow_data()?;
    let mut fixed_data = [0u8; 2];
    fixed_data.copy_from_slice(&instruction_sysvar[0..2]);
    let num_instructions = u16::from_le_bytes(fixed_data);
    if num_instructions > 2 {
        assert!(false, "Suspicious transaction, failing")
    } else if num_instructions == 2 {
        let compute_budget_instruction =
            load_instruction_at_checked(0, instruction_sysvar_account)?;

        let compute_budget_id: Pubkey =
            Pubkey::new(bs58::decode(&COMPUTE_BUDGET_ADDRESS).into_vec().unwrap()[..32].as_ref());

        assert_eq!(compute_budget_instruction.program_id, compute_budget_id);
        let current_instruction = load_instruction_at_checked(1, instruction_sysvar_account)?;
        assert_eq!(current_instruction.program_id, id());
    } else if num_instructions == 1 {
        let only_instruction = load_instruction_at_checked(0, instruction_sysvar_account)?;
        assert_eq!(only_instruction.program_id, id());
    }
    // We should not be executing dispense... from a CPI

    return Ok(());
}

#[inline(always)]
// Preform a fisher_yates shuffle on the array of indices into the config lines data structure. Then return the
// metadata args corresponding to the chosen config line
fn fisher_yates_shuffle_and_fetch_nft_metadata<'info>(
    recent_blockhashes: &UncheckedAccount<'info>,
    gumball_header: &mut GumballMachineHeader,
    indices: &mut [u32],
    line_size: usize,
    config_lines_data: &mut [u8],
) -> Result<MetadataArgs> {
    // Get 8 bytes of entropy from the SlotHashes sysvar
    let mut buf: [u8; 4] = [0; 4];
    buf.copy_from_slice(
        &hashv(&[
            &recent_blockhashes.data.borrow(),
            &(gumball_header.remaining).to_le_bytes(),
        ])
        .as_ref()[..4],
    );
    let entropy = u32::from_le_bytes(buf);
    // Shuffle the list of indices using Fisher-Yates
    let selected = entropy % gumball_header.remaining;
    gumball_header.remaining -= 1;
    indices.swap(selected as usize, gumball_header.remaining as usize);
    // Pull out config line from the data
    let zero_nft_index = indices[(gumball_header.remaining as usize)] as usize;
    let nft_index = zero_nft_index + 1;
    msg!("Minted NFT with 1-index {}", nft_index);
    let random_config_index = zero_nft_index * line_size;
    let config_line = if config_lines_data.len() > 0 {
        // If the machine is manually specifying config lines then extract the config line data
        config_lines_data[random_config_index..random_config_index + line_size].to_vec()
    } else {
        // Otherwise the 1-index serves as the config line
        nft_index.to_le_bytes().to_vec()
    };

    let message = get_metadata_args(
        gumball_header.url_base,
        gumball_header.name_base,
        gumball_header.symbol,
        gumball_header.seller_fee_basis_points,
        gumball_header.is_mutable != 0,
        gumball_header.collection_key,
        None,
        gumball_header.creators,
        nft_index,
        config_line,
        EncodeMethod::from(gumball_header.config_line_encode_method),
    );
    return Ok(message);
}

#[inline(always)]
// For efficiency, this returns the GumballMachineHeader because it's required to validate
// payment parameters. But the main purpose of this function is to determine which config
// line to mint to the user, and CPI to bubblegum to actually execute the mint
// Also returns the number of nfts successfully minted, so that the purchaser is charged
// appropriately
fn find_and_mint_compressed_nfts<'info>(
    gumball_machine: &AccountInfo<'info>,
    payer: &Signer<'info>,
    willy_wonka: &AccountInfo<'info>,
    willy_wonka_bump: &u8,
    recent_blockhashes: &UncheckedAccount<'info>,
    instruction_sysvar_account: &AccountInfo<'info>,
    bubblegum_authority: &AccountInfo<'info>,
    gummyroll: &Program<'info, Gummyroll>,
    merkle_slab: &AccountInfo<'info>,
    bubblegum: &Program<'info, Bubblegum>,
    candy_wrapper_program: &Program<'info, CandyWrapper>,
    num_items: u32,
) -> Result<(GumballMachineHeader, u32)> {
    // Prevent atomic transaction exploit attacks
    // TODO: potentially record information about botting now as pretains to payments to bot_wallet
    assert_valid_single_instruction_transaction(instruction_sysvar_account)?;

    // Load all data
    let mut gumball_machine_data = gumball_machine.try_borrow_mut_data()?;
    let (mut header_bytes, config_data) =
        gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
    let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;

    // Cannot dispense before all indices are initialized
    assert_eq!(
        gumball_header.max_items,
        gumball_header.smallest_uninitialized_index
    );

    // Cannot dispense more than the max_mint_size
    assert!(num_items <= gumball_header.max_mint_size);

    // Cannot dispense before project is live
    let clock = Clock::get()?;
    assert!(clock.unix_timestamp > gumball_header.go_live_date);

    let size = gumball_header.max_items as usize;
    let index_array_size = std::mem::size_of::<u32>() * size;
    let config_size = gumball_header.extension_len as usize * size;
    let line_size = gumball_header.extension_len as usize;

    assert!(config_data.len() == index_array_size + config_size);
    let (indices_data, config_lines_data) = config_data.split_at_mut(index_array_size);

    // TODO: Validate data

    let indices = cast_slice_mut::<u8, u32>(indices_data);
    let num_nfts_to_mint: u32 = (num_items).max(1).min(gumball_header.remaining);
    assert!(
        num_nfts_to_mint > 0,
        "There are no remaining NFTs to dispense!"
    );
    for _ in 0..num_nfts_to_mint {
        let message = fisher_yates_shuffle_and_fetch_nft_metadata(
            recent_blockhashes,
            gumball_header,
            indices,
            line_size,
            config_lines_data,
        )?;

        let seed = gumball_machine.key();
        let seeds = &[seed.as_ref(), &[*willy_wonka_bump]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            bubblegum.to_account_info(),
            bubblegum::cpi::accounts::MintV1 {
                mint_authority: willy_wonka.to_account_info(),
                authority: bubblegum_authority.to_account_info(),
                candy_wrapper: candy_wrapper_program.to_account_info(),
                gummyroll_program: gummyroll.to_account_info(),
                owner: payer.to_account_info(),
                delegate: payer.to_account_info(),
                merkle_slab: merkle_slab.to_account_info(),
            },
            authority_pda_signer,
        );
        bubblegum::cpi::mint_v1(cpi_ctx, message)?;
    }
    Ok((*gumball_header, num_nfts_to_mint))
}

#[program]
pub mod gumball_machine {
    use super::*;

    /// Initialize Gumball Machine header properties, and initialize downstream data structures (Gummyroll tree)
    pub fn initialize_gumball_machine(
        ctx: Context<InitGumballMachine>,
        max_depth: u32,
        max_buffer_size: u32,
        url_base: [u8; 64],
        name_base: [u8; 32],
        symbol: [u8; 8],
        encode_method: Option<EncodeMethod>,
        seller_fee_basis_points: u16,
        is_mutable: bool,
        retain_authority: bool,
        price: u64,
        go_live_date: i64,
        bot_wallet: Pubkey,
        receiver: Pubkey,
        authority: Pubkey,
        collection_key: Pubkey,
        extension_len: u64,
        max_mint_size: u32,
        max_items: u32,
        creator_keys: Vec<Pubkey>,
        creator_shares: Vec<u8>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = max_items as usize;

        // Construct creators array
        let mut creators: [GumballCreatorAdapter; NUM_CREATORS] = [
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ];
        assert_eq!(creator_keys.len(), creator_shares.len());
        assert!(
            creator_keys.len() < NUM_CREATORS,
            "Cannot set more than {} creators",
            NUM_CREATORS
        );
        assert!(
            creator_shares.len() == 0 || creator_shares.iter().sum::<u8>() == 100,
            "If specifying creators, shares must sum to 100% of royalty allocation."
        );
        for i in 0..creator_keys.len() {
            let creator_to_add = GumballCreatorAdapter {
                address: creator_keys[i],
                // TODO: metaplex is working on creator verification
                verified: (0 as u8),
                share: creator_shares[i],
            };
            creators[i] = creator_to_add;
        }
        *gumball_header = GumballMachineHeader {
            url_base: url_base,
            name_base: name_base,
            symbol: symbol,
            seller_fee_basis_points,
            is_mutable: is_mutable.into(),
            retain_authority: retain_authority.into(),
            config_line_encode_method: match encode_method {
                Some(e) => e.to_u8(),
                None => EncodeMethod::UTF8.to_u8(),
            },
            creators,
            _padding: [0; 1],
            price,
            go_live_date,
            bot_wallet,
            receiver,
            authority,
            mint: ctx.accounts.mint.key(),
            collection_key,
            extension_len: extension_len,
            max_mint_size: max_mint_size.max(1).min(max_items),
            remaining: 0,
            max_items,
            total_items_added: 0,
            smallest_uninitialized_index: 0,
            _padding_2: [0; 4],
        };
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = extension_len as usize * size;
        assert!(config_data.len() == index_array_size + config_size);
        let seed = ctx.accounts.gumball_machine.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("willy_wonka").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum.to_account_info(),
            bubblegum::cpi::accounts::CreateTree {
                tree_creator: ctx.accounts.willy_wonka.to_account_info(),
                authority: ctx.accounts.bubblegum_authority.to_account_info(),
                candy_wrapper: ctx.accounts.candy_wrapper.to_account_info(),
                gummyroll_program: ctx.accounts.gummyroll.to_account_info(),
                merkle_slab: ctx.accounts.merkle_slab.to_account_info(),
                payer: ctx.accounts.payer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            authority_pda_signer,
        );
        bubblegum::cpi::create_tree(cpi_ctx, max_depth, max_buffer_size)
    }

    /// Initialize chunk of NFT indices (as many as possible within the compute budget of a single transaction). All indices must be initialized before the tree can dispense.
    pub fn initialize_indices_chunk(ctx: Context<InitIndices>) -> Result<()> {
        // Fetch mutable header data
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let mut gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;

        // Assert that indices initialization is authorized
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());

        // Grab mutable reference to indices bytes
        let size = gumball_header.max_items as usize;
        let index_array_size = std::mem::size_of::<u32>() * size;
        let (indices_data, _) = config_data.split_at_mut(index_array_size);
        let indices = cast_slice_mut::<u8, u32>(indices_data);

        // Determine the next byte range to initialize
        let first_index_to_initialize = gumball_header.smallest_uninitialized_index as usize;
        let next_smallest_uninitialized_index = (gumball_header.smallest_uninitialized_index
            + MAX_NUM_INDICES_TO_INIT_FOR_CHUNK)
            .min(gumball_header.max_items) as usize;
        indices[first_index_to_initialize..next_smallest_uninitialized_index]
            .iter_mut()
            .enumerate()
            .for_each(|(i, idx)| *idx = (i + first_index_to_initialize) as u32);
        msg!(
            "Initialized indices {} up to and not including {}",
            first_index_to_initialize,
            next_smallest_uninitialized_index
        );
        gumball_header.smallest_uninitialized_index = next_smallest_uninitialized_index as u32;

        // If the machine is not using config lines, and has fully initialized its inidices then we set its remaining NFTs to max
        if gumball_header.extension_len == 0 {
            gumball_header.remaining = gumball_header.max_items;
        }
        Ok(())
    }

    /// Add can only append config lines to the the end of the list
    pub fn add_config_lines(
        ctx: Context<UpdateConfigLine>,
        new_config_lines_data: Vec<u8>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let mut gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = gumball_header.max_items as usize;
        let index_array_size = std::mem::size_of::<u32>() * size;
        let line_size = gumball_header.extension_len as usize;
        let num_lines = new_config_lines_data.len() / line_size;
        let start_index = gumball_header.total_items_added as usize;
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());
        assert_eq!(new_config_lines_data.len() % line_size, 0);
        assert!(start_index + num_lines <= gumball_header.max_items as usize);
        let (_, config_lines_data) = config_data.split_at_mut(index_array_size);
        config_lines_data[start_index..]
            .iter_mut()
            .take(new_config_lines_data.len())
            .enumerate()
            .for_each(|(i, l)| *l = new_config_lines_data[i]);
        gumball_header.total_items_added += num_lines as u32;
        gumball_header.remaining += num_lines as u32;
        Ok(())
    }

    /// Update only allows the authority to modify previously appended lines
    pub fn update_config_lines(
        ctx: Context<UpdateConfigLine>,
        starting_line: u64,
        new_config_lines_data: Vec<u8>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = gumball_header.max_items as usize;
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = gumball_header.extension_len as usize * size;
        let line_size = gumball_header.extension_len as usize;
        let num_lines = new_config_lines_data.len() / line_size;
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());
        assert_eq!(new_config_lines_data.len() % line_size, 0);
        assert!(config_data.len() == index_array_size + config_size);
        assert_eq!(new_config_lines_data.len(), num_lines * line_size);
        assert!(starting_line as usize + num_lines <= gumball_header.total_items_added as usize);
        let (_, config_lines_data) = config_data.split_at_mut(index_array_size);
        config_lines_data[starting_line as usize * line_size..]
            .iter_mut()
            .take(new_config_lines_data.len())
            .enumerate()
            .for_each(|(i, l)| *l = new_config_lines_data[i]);
        Ok(())
    }

    pub fn update_header_metadata(
        ctx: Context<UpdateHeaderMetadata>,
        url_base: Option<[u8; 64]>,
        name_base: Option<[u8; 32]>,
        symbol: Option<[u8; 8]>,
        encode_method: Option<EncodeMethod>,
        seller_fee_basis_points: Option<u16>,
        is_mutable: Option<bool>,
        retain_authority: Option<bool>,
        price: Option<u64>,
        go_live_date: Option<i64>,
        bot_wallet: Option<Pubkey>,
        authority: Option<Pubkey>,
        receiver: Option<Pubkey>,
        max_mint_size: Option<u32>,
        creator_keys: Option<Vec<Pubkey>>,
        creator_shares: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, _) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let mut gumball_machine = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        assert_eq!(gumball_machine.authority, ctx.accounts.authority.key());
        match url_base {
            Some(ub) => gumball_machine.url_base = ub,
            None => {}
        }
        match name_base {
            Some(nb) => gumball_machine.name_base = nb,
            None => {}
        }
        match symbol {
            Some(s) => gumball_machine.symbol = s,
            None => {}
        }
        match encode_method {
            Some(e) => gumball_machine.config_line_encode_method = e.to_u8(),
            None => {}
        }
        // TODO: consider this. Could result in unexpectedly high fees upon secondary sales if this is modified after the project goes live, but before all NFTs are minted.
        //       maybe we gate this against project go live date?
        match seller_fee_basis_points {
            Some(s) => gumball_machine.seller_fee_basis_points = s,
            None => {}
        }
        match is_mutable {
            Some(im) => gumball_machine.is_mutable = im.into(),
            None => {}
        }
        match retain_authority {
            Some(ra) => gumball_machine.retain_authority = ra.into(),
            None => {}
        }
        // TODO: consider this. Could result in unexpectedly high prices if this is modified after the project goes live, but before all NFTs are minted.
        //       maybe we gate this against project go live date?
        match price {
            Some(p) => gumball_machine.price = p,
            None => {}
        }
        match go_live_date {
            Some(gld) => gumball_machine.go_live_date = gld, // TODO: Are we worried about clock drift and ppl trying to hit the machine close to when this goes live, and projects updating close to go live date? Consider changing to be slothash, etc.
            None => {}
        }
        match authority {
            Some(a) => gumball_machine.authority = a,
            None => {}
        }
        match bot_wallet {
            Some(bw) => gumball_machine.bot_wallet = bw,
            None => {}
        }
        match receiver {
            Some(r) => gumball_machine.receiver = r,
            None => {}
        }
        match max_mint_size {
            Some(mms) => gumball_machine.max_mint_size = mms.max(1).min(gumball_machine.max_items),
            None => {}
        }
        match creator_keys {
            Some(cks) => {
                // If creator_shares is None but creator_keys is specified, input is invalid -> panic
                let cs = creator_shares.unwrap();
                assert!(
                    cs.len() == 0 || cs.iter().sum::<u8>() == 100,
                    "If specifying creators, shares must sum to 100% of royalty allocation."
                );
                assert_eq!(cks.len(), cs.len());
                assert!(
                    cks.len() < NUM_CREATORS,
                    "Cannot set more than {} creators",
                    NUM_CREATORS
                );
                // Construct creators array
                let mut creators: [GumballCreatorAdapter; NUM_CREATORS] = [
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ];
                for i in 0..cks.len() {
                    let creator_to_add = GumballCreatorAdapter {
                        address: cks[i],
                        // TODO: metaplex is working on creator verification
                        verified: (0 as u8),
                        share: cs[i],
                    };
                    creators[i] = creator_to_add;
                }
                // Overwrite existing creators array, note all creators must then be re-verified
                gumball_machine.creators = creators;
            },
            None => {}
        }
        Ok(())
    }

    /// Request to purchase a random NFT from GumballMachine for a specific project.
    /// @notice: the project must have specified the native mint (Wrapped SOL) for "mint"
    ///          in its GumballMachineHeader for this method to succeed. If mint is anything
    ///          else dispense_nft_token should be used.
    pub fn dispense_nft_sol(ctx: Context<DispenseSol>, num_items: u32) -> Result<()> {
        let (gumball_header, num_nfts_minted) = find_and_mint_compressed_nfts(
            &ctx.accounts.gumball_machine,
            &ctx.accounts.payer,
            &ctx.accounts.willy_wonka,
            ctx.bumps.get("willy_wonka").unwrap(),
            &ctx.accounts.recent_blockhashes,
            &ctx.accounts.instruction_sysvar_account,
            &ctx.accounts.bubblegum_authority,
            &ctx.accounts.gummyroll,
            &ctx.accounts.merkle_slab,
            &ctx.accounts.bubblegum,
            &ctx.accounts.candy_wrapper,
            num_items,
        )?;

        // Process payment for NFT
        assert_eq!(&gumball_header.receiver.key(), &ctx.accounts.receiver.key());

        // Can only use this instruction for projects seeking SOL
        let wrapped_sol_pubkey: Pubkey = native_mint::ID;
        assert_eq!(gumball_header.mint, wrapped_sol_pubkey);

        invoke(
            &system_instruction::transfer(
                &ctx.accounts.payer.key(),
                &ctx.accounts.receiver.key(),
                gumball_header.price * (num_nfts_minted as u64),
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.receiver.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        Ok(())
    }

    /// Request to purchase a random NFT from GumballMachine for a specific project.
    /// @notice: the project's mint may be any valid Mint account EXCEPT for Wrapped SOL
    ///          if the mint is Wrapped SOL then dispense_token_sol should be used, as the
    ///          project is seeking native SOL as payment.
    pub fn dispense_nft_token(ctx: Context<DispenseToken>, num_items: u32) -> Result<()> {
        let (gumball_header, num_nfts_minted) = find_and_mint_compressed_nfts(
            &ctx.accounts.gumball_machine,
            &ctx.accounts.payer,
            &ctx.accounts.willy_wonka,
            ctx.bumps.get("willy_wonka").unwrap(),
            &ctx.accounts.recent_blockhashes,
            &ctx.accounts.instruction_sysvar_account,
            &ctx.accounts.bubblegum_authority,
            &ctx.accounts.gummyroll,
            &ctx.accounts.merkle_slab,
            &ctx.accounts.bubblegum,
            &ctx.accounts.candy_wrapper,
            num_items,
        )?;

        // Process payment for NFT
        assert_eq!(&gumball_header.receiver.key(), &ctx.accounts.receiver.key());
        assert_eq!(ctx.accounts.payer_tokens.mint, gumball_header.mint);
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.payer_tokens.to_account_info(),
                    to: ctx.accounts.receiver.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            gumball_header.price * (num_nfts_minted as u64),
        )?;
        Ok(())
    }

    /// Reclaim gumball_machine lamports to authority
    pub fn destroy(ctx: Context<Destroy>) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, _) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        assert!(gumball_header.authority == ctx.accounts.authority.key());
        let dest_starting_lamports = ctx.accounts.authority.lamports();
        **ctx.accounts.authority.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(ctx.accounts.gumball_machine.lamports())
            .ok_or(ProgramError::InvalidAccountData)?;
        **ctx.accounts.gumball_machine.lamports.borrow_mut() = 0;
        Ok(())
    }
}
