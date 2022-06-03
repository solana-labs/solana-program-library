use anchor_lang::{
    prelude::*,
    solana_program::{
        keccak::hashv, 
        sysvar,
        sysvar::instructions::{load_instruction_at_checked}, 
        sysvar::SysvarId, 
        pubkey::Pubkey, 
        program::{invoke, invoke_signed},
        system_instruction,
        instruction::{Instruction},
    },
};
use anchor_spl::token::{Mint, TokenAccount, Token, Transfer, transfer};
use spl_token::native_mint;
use bubblegum::program::Bubblegum;
use bubblegum::state::metaplex_adapter::UseMethod;
use bubblegum::state::metaplex_adapter::Uses;
use bubblegum::state::leaf_schema::Version;
use bytemuck::cast_slice_mut;
use gummyroll::program::Gummyroll;
pub mod state;
pub mod utils;

use crate::state::{GumballMachineHeader, ZeroCopy};
use crate::utils::get_metadata_args;

declare_id!("BRKyVDRGT7SPBtMhjHN4PVSPVYoc3Wa3QTyuRVM4iZkt");

#[derive(Accounts)]
pub struct InitGumballMachine<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(zero)]
    gumball_machine: AccountInfo<'info>,
    creator: Signer<'info>,
    mint: Account<'info, Mint>,
    /// CHECK: Mint/append authority to the merkle slab
    #[account(
        seeds = [gumball_machine.key().as_ref()],
        bump,
    )]
    willy_wonka: AccountInfo<'info>,
    /// CHECK: Tree authority to the merkle slab, PDA owned by BubbleGum
    bubblegum_authority: AccountInfo<'info>,
    gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Empty merkle slab
    #[account(zero)]
    merkle_slab: AccountInfo<'info>,
    bubblegum: Program<'info, Bubblegum>,
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
    bubblegum_authority: AccountInfo<'info>,
    /// CHECK: PDA is checked in Bubblegum
    #[account(mut)]
    nonce: AccountInfo<'info>,
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
    bubblegum_authority: AccountInfo<'info>,
    /// CHECK: PDA is checked in Bubblegum
    #[account(mut)]
    nonce: AccountInfo<'info>,
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
fn assert_valid_single_instruction_transaction<'info>(instruction_sysvar_account: &AccountInfo<'info>) -> Result<()> {
    // There should only be one instruction in this transaction (the current call to dispense_...)
    let instruction_sysvar = instruction_sysvar_account.try_borrow_data()?;
    let mut fixed_data = [0u8; 2];
    fixed_data.copy_from_slice(&instruction_sysvar[0..2]);
    let num_instructions = u16::from_le_bytes(fixed_data);
    assert_eq!(num_instructions, 1);

    // We should not be executing dispense... from a CPI
    let only_instruction = load_instruction_at_checked(0, instruction_sysvar_account)?;
    assert_eq!(only_instruction.program_id, id());
    return Ok(())
}

#[inline(always)]
// For efficiency, this returns the GumballMachineHeader because it's required to validate
// payment parameters. But the main purpose of this function is to determine which config
// line to mint to the user, and CPI to bubblegum to actually execute the mint
fn find_and_mint_compressed_nft<'info>(
    gumball_machine: &AccountInfo<'info>,
    payer: &Signer<'info>,
    willy_wonka: &AccountInfo<'info>,
    willy_wonka_bump: &u8,
    recent_blockhashes: &UncheckedAccount<'info>,
    instruction_sysvar_account: &AccountInfo<'info>,
    bubblegum_authority: &AccountInfo<'info>,
    nonce: &AccountInfo<'info>,
    gummyroll: &Program<'info, Gummyroll>,
    merkle_slab: &AccountInfo<'info>,
    bubblegum: &Program<'info, Bubblegum>,
    num_items: u64
) -> Result<GumballMachineHeader> {
    
    // Prevent atomic transaction exploit attacks
    // TODO: potentially record information about botting now as pretains to payments to bot_wallet
    assert_valid_single_instruction_transaction(instruction_sysvar_account)?;

    // Load all data
    let mut gumball_machine_data = gumball_machine.try_borrow_mut_data()?;
    let (mut header_bytes, config_data) =
        gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
    let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
    let clock = Clock::get()?;
    assert!(clock.unix_timestamp > gumball_header.go_live_date);
    let size = gumball_header.max_items as usize;
    let index_array_size = std::mem::size_of::<u32>() * size;
    let config_size = gumball_header.extension_len * size;
    let line_size = gumball_header.extension_len;

    assert!(config_data.len() == index_array_size + config_size);
    let (indices_data, config_lines_data) = config_data.split_at_mut(index_array_size);

    // TODO: Validate data

    let mut indices = cast_slice_mut::<u8, u32>(indices_data);
    for _ in 0..(num_items as usize).max(1).min(gumball_header.remaining) {
        // Get 8 bytes of entropy from the SlotHashes sysvar
        let mut buf: [u8; 8] = [0; 8];
        buf.copy_from_slice(
            &hashv(&[
                &recent_blockhashes.data.borrow(),
                &gumball_header.remaining.to_le_bytes(),
            ])
            .as_ref()[..8],
        );
        let entropy = u64::from_le_bytes(buf);
        // Shuffle the list of indices using Fisher-Yates
        let selected = entropy % gumball_header.remaining as u64;
        gumball_header.remaining -= 1;
        (&mut indices).swap(selected as usize, gumball_header.remaining);
        // Pull out config line from the data
        let random_config_index = indices[gumball_header.remaining] as usize * line_size;
        let config_line =
            config_lines_data[random_config_index..random_config_index + line_size].to_vec();

        let message = get_metadata_args(
            gumball_header.url_base,
            gumball_header.name_base,
            gumball_header.symbol,
            gumball_header.seller_fee_basis_points,
            gumball_header.is_mutable != 0,
            gumball_header.collection_key,
            None,
            gumball_header.creator_address,
            random_config_index,
            config_line,
        );

        let seed = gumball_machine.key();
        let seeds = &[seed.as_ref(), &[*willy_wonka_bump]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            bubblegum.to_account_info(),
            bubblegum::cpi::accounts::Mint {
                mint_authority: willy_wonka.to_account_info(),
                authority: bubblegum_authority.to_account_info(),
                nonce: nonce.to_account_info(),
                gummyroll_program: gummyroll.to_account_info(),
                owner: payer.to_account_info(),
                delegate: payer.to_account_info(),
                merkle_slab: merkle_slab.to_account_info(),
            },
            authority_pda_signer,
        );
        bubblegum::cpi::mint(cpi_ctx, Version::V0, message)?;
    }
    Ok(*gumball_header)
}

#[program]
pub mod gumball_machine {
    use super::*;

    // TODO(sorend): consider validating receiver in here. I.e. forcing the receiver to be the
    // associated token account of creator_address and mint. This restricts payment reciept options,
    // but it allows validation that all initialized gumball machines can receive payment
    pub fn initialize_gumball_machine(
        ctx: Context<InitGumballMachine>,
        max_depth: u32,
        max_buffer_size: u32,
        url_base: [u8; 64],
        name_base: [u8; 32],
        symbol: [u8; 32],
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
        max_mint_size: u64,
        max_items: u64,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = max_items as usize;
        *gumball_header = GumballMachineHeader {
            url_base: url_base,
            name_base: name_base,
            symbol: symbol,
            seller_fee_basis_points,
            is_mutable: is_mutable.into(),
            retain_authority: retain_authority.into(),
            _padding: [0; 4],
            price,
            go_live_date,
            bot_wallet,
            receiver,
            authority,
            mint: ctx.accounts.mint.key(),
            collection_key,
            creator_address: ctx.accounts.creator.key(),
            extension_len: extension_len as usize,
            max_mint_size: max_mint_size.max(1).min(max_items),
            remaining: 0,
            max_items,
            total_items_added: 0,
        };
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = extension_len as usize * size;
        assert!(config_data.len() == index_array_size + config_size);
        let (indices_data, _) = config_data.split_at_mut(index_array_size);
        let indices = cast_slice_mut::<u8, u32>(indices_data);
        indices
            .iter_mut()
            .enumerate()
            .for_each(|(i, idx)| *idx = i as u32);
        let seed = ctx.accounts.gumball_machine.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("willy_wonka").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum.to_account_info(),
            bubblegum::cpi::accounts::CreateTree {
                tree_creator: ctx.accounts.willy_wonka.to_account_info(),
                authority: ctx.accounts.bubblegum_authority.to_account_info(),
                gummyroll_program: ctx.accounts.gummyroll.to_account_info(),
                merkle_slab: ctx.accounts.merkle_slab.to_account_info(),
            },
            authority_pda_signer,
        );
        bubblegum::cpi::create_tree(cpi_ctx, max_depth, max_buffer_size)
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
        let config_size = gumball_header.extension_len * size;
        let line_size = gumball_header.extension_len;
        let num_lines = new_config_lines_data.len() / line_size; // unchecked divide by zero? maybe we don't care since this will throw and the instr will fail
        let start_index = gumball_header.total_items_added;
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());
        assert_eq!(new_config_lines_data.len() % line_size, 0);
        assert!(start_index + num_lines <= gumball_header.max_items as usize);
        let (_, config_lines_data) = config_data.split_at_mut(index_array_size);
        config_lines_data[start_index..]
            .iter_mut()
            .take(new_config_lines_data.len())
            .enumerate()
            .for_each(|(i, l)| *l = new_config_lines_data[i]);
        gumball_header.total_items_added += num_lines;
        gumball_header.remaining += num_lines;
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
        let config_size = gumball_header.extension_len * size;
        let line_size = gumball_header.extension_len;
        let num_lines = new_config_lines_data.len() / line_size; // unchecked divide by zero? maybe we don't care since this will throw and the instr will fail
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());
        assert_eq!(new_config_lines_data.len() % line_size, 0);
        assert!(config_data.len() == index_array_size + config_size);
        assert_eq!(new_config_lines_data.len(), num_lines * line_size);
        assert!(starting_line as usize + num_lines <= gumball_header.total_items_added);
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
        symbol: Option<[u8; 32]>,
        seller_fee_basis_points: Option<u16>,
        is_mutable: Option<bool>,
        retain_authority: Option<bool>,
        price: Option<u64>,
        go_live_date: Option<i64>,
        bot_wallet: Option<Pubkey>,
        authority: Option<Pubkey>,
        max_mint_size: Option<u64>,
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
        // TODO(sorend): consider allowing changes to receiver, requires validation of receiver
        match max_mint_size {
            Some(mms) => gumball_machine.max_mint_size = mms.max(1).min(gumball_machine.max_items),
            None => {}
        }
        Ok(())
    }

    /// Request to purchase a random NFT from GumballMachine for a specific project.
    /// @notice: the project must have specified the native mint (Wrapped SOL) for "mint" 
    ///          in its GumballMachineHeader for this method to succeed. If mint is anything
    ///          else dispense_nft_token should be used.
    pub fn dispense_nft_sol(ctx: Context<DispenseSol>, num_items: u64) -> Result<()> {
        let gumball_header = find_and_mint_compressed_nft(
            &ctx.accounts.gumball_machine,
            &ctx.accounts.payer,
            &ctx.accounts.willy_wonka,
            ctx.bumps.get("willy_wonka").unwrap(),
            &ctx.accounts.recent_blockhashes,
            &ctx.accounts.instruction_sysvar_account,
            &ctx.accounts.bubblegum_authority,
            &ctx.accounts.nonce,
            &ctx.accounts.gummyroll,
            &ctx.accounts.merkle_slab,
            &ctx.accounts.bubblegum,
            num_items
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
                gumball_header.price
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.receiver.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ]
        )?;

        Ok(())
    }

    /// Request to purchase a random NFT from GumballMachine for a specific project.
    /// @notice: the project's mint may be any valid Mint account EXCEPT for Wrapped SOL
    ///          if the mint is Wrapped SOL then dispense_token_sol should be used, as the
    ///          project is seeking native SOL as payment.
    pub fn dispense_nft_token(ctx: Context<DispenseToken>, num_items: u64) -> Result<()> {
        let gumball_header = find_and_mint_compressed_nft(
            &ctx.accounts.gumball_machine,
            &ctx.accounts.payer,
            &ctx.accounts.willy_wonka,
            ctx.bumps.get("willy_wonka").unwrap(),
            &ctx.accounts.recent_blockhashes,
            &ctx.accounts.instruction_sysvar_account,
            &ctx.accounts.bubblegum_authority,
            &ctx.accounts.nonce,
            &ctx.accounts.gummyroll,
            &ctx.accounts.merkle_slab,
            &ctx.accounts.bubblegum,
            num_items
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
                }
            ),
            gumball_header.price
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
