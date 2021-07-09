use crate::{
    error::BettingPoolError,
    instruction::BettingPoolInstruction,
    spl_utils::{
        spl_burn, spl_initialize, spl_mint_initialize, spl_mint_to, spl_set_authority,
        spl_token_transfer, spl_token_transfer_signed, spl_approve, spl_burn_signed,
    },
    state::BettingPool,
    system_utils::{create_new_account, create_or_allocate_account_raw, topup},
    validation_utils::{
        assert_initialized, assert_keys_equal, assert_keys_unequal,
        assert_mint_authority_matches_mint, assert_owned_by,
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::{
    instruction::AuthorityType,
    state::{Account, Mint},
};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = BettingPoolInstruction::try_from_slice(instruction_data)?;
        match instruction {
            BettingPoolInstruction::InitializeBettingPool(args) => {
                msg!("Instruction: InitializeBettingPool");
                process_initialize_betting_pool(program_id, accounts, args.decimals)
            }
            BettingPoolInstruction::Trade(args) => {
                msg!("Instruction: Trade");
                process_trade(
                    program_id,
                    accounts,
                    args.size,
                    args.buy_price,
                    args.sell_price,
                )
            }
            BettingPoolInstruction::Settle => {
                msg!("Instruction: Settle");
                process_settle(program_id, accounts)
            }
            BettingPoolInstruction::Collect => {
                msg!("Instruction: Collect");
                process_collect(program_id, accounts)
            }
        }
    }
}

pub fn process_initialize_betting_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    decimals: u8,
) -> ProgramResult {
    msg!("InitializeBettingPool");
    let account_info_iter = &mut accounts.iter();
    let pool_account_info = next_account_info(account_info_iter)?;
    let escrow_mint_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    let long_token_mint_info = next_account_info(account_info_iter)?;
    let short_token_mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let update_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    create_new_account(
        &mint_authority_info,
        &long_token_mint_info,
        Mint::LEN,
        &token_program_info,
        &rent_info,
    )?;
    create_new_account(
        &mint_authority_info,
        &short_token_mint_info,
        Mint::LEN,
        &token_program_info,
        &rent_info,
    )?;
    create_new_account(
        &update_authority_info,
        &escrow_account_info,
        Account::LEN,
        &token_program_info,
        &rent_info,
    )?;
    spl_mint_initialize(
        &token_program_info,
        &long_token_mint_info,
        &mint_authority_info,
        &mint_authority_info,
        &rent_info,
        0,
    )?;
    spl_mint_initialize(
        &token_program_info,
        &short_token_mint_info,
        &mint_authority_info,
        &mint_authority_info,
        &rent_info,
        0,
    )?;
    spl_initialize(
        &token_program_info,
        &escrow_account_info,
        &escrow_mint_info,
        &update_authority_info,
        &rent_info,
    )?;

    let long_token_mint: Mint = assert_initialized(long_token_mint_info)?;
    let short_token_mint: Mint = assert_initialized(short_token_mint_info)?;
    let escrow_account: Account = assert_initialized(escrow_account_info)?;

    assert_mint_authority_matches_mint(&long_token_mint, mint_authority_info)?;
    assert_mint_authority_matches_mint(&short_token_mint, mint_authority_info)?;
    assert_owned_by(long_token_mint_info, &spl_token::id())?;
    assert_owned_by(short_token_mint_info, &spl_token::id())?;
    assert_keys_equal(*token_program_info.key, spl_token::id())?;

    assert_keys_equal(escrow_account.mint, *escrow_mint_info.key)?;

    // Transfer ownership of the escrow accounts to a PDA
    let (authority_key, _) = Pubkey::find_program_address(
        &[
            long_token_mint_info.key.as_ref(),
            short_token_mint_info.key.as_ref(),
            token_program_info.key.as_ref(),
            program_id.as_ref(),
        ],
        program_id,
    );
    spl_set_authority(
        token_program_info,
        escrow_account_info,
        Some(authority_key),
        AuthorityType::AccountOwner,
        update_authority_info,
    )?;
    spl_set_authority(
        token_program_info,
        long_token_mint_info,
        Some(authority_key),
        AuthorityType::MintTokens,
        update_authority_info,
    )?;
    spl_set_authority(
        token_program_info,
        short_token_mint_info,
        Some(authority_key),
        AuthorityType::MintTokens,
        update_authority_info,
    )?;

    create_or_allocate_account_raw(
        *program_id,
        pool_account_info,
        rent_info,
        system_account_info,
        update_authority_info,
        BettingPool::LEN,
        &[],
    )?;

    let mut betting_pool = BettingPool::try_from_slice(&pool_account_info.data.borrow_mut())?;
    betting_pool.decimals = decimals;
    betting_pool.circulation = 0;
    betting_pool.settled = false;
    betting_pool.long_mint_account_pubkey = *long_token_mint_info.key;
    betting_pool.short_mint_account_pubkey = *short_token_mint_info.key;
    betting_pool.escrow_mint_account_pubkey = *escrow_mint_info.key;
    betting_pool.escrow_account_pubkey = *escrow_account_info.key;
    betting_pool.owner = *update_authority_info.key;
    betting_pool.serialize(&mut *pool_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_trade(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    size: u64,
    buy_price: u64,
    sell_price: u64,
) -> ProgramResult {
    msg!("Trade");
    let account_info_iter = &mut accounts.iter();
    let pool_account_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    let long_token_mint_info = next_account_info(account_info_iter)?;
    let short_token_mint_info = next_account_info(account_info_iter)?;
    let buyer_info = next_account_info(account_info_iter)?;
    let seller_info = next_account_info(account_info_iter)?;
    let buyer_account_info = next_account_info(account_info_iter)?;
    let seller_account_info = next_account_info(account_info_iter)?;
    let buyer_long_token_account_info = next_account_info(account_info_iter)?;
    let buyer_short_token_account_info = next_account_info(account_info_iter)?;
    let seller_long_token_account_info = next_account_info(account_info_iter)?;
    let seller_short_token_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    // Unpack accounts
    let long_token_mint: Mint = assert_initialized(long_token_mint_info)?;
    let short_token_mint: Mint = assert_initialized(short_token_mint_info)?;
    let buyer_long_token_account: Account = assert_initialized(buyer_long_token_account_info)?;
    let buyer_short_token_account: Account = assert_initialized(buyer_short_token_account_info)?;
    let seller_long_token_account: Account = assert_initialized(seller_long_token_account_info)?;
    let seller_short_token_account: Account = assert_initialized(seller_short_token_account_info)?;
    let buyer_account: Account = assert_initialized(buyer_account_info)?;
    let seller_account: Account = assert_initialized(seller_account_info)?;
    let mut betting_pool = BettingPool::try_from_slice(&pool_account_info.data.borrow_mut())?;

    // Get program derived address for escrow
    let (authority_key, bump_seed) = Pubkey::find_program_address(
        &[
            long_token_mint_info.key.as_ref(),
            short_token_mint_info.key.as_ref(),
            token_program_info.key.as_ref(),
            program_id.as_ref(),
        ],
        program_id,
    );
    let seeds = &[
        long_token_mint_info.key.as_ref(),
        short_token_mint_info.key.as_ref(),
        token_program_info.key.as_ref(),
        program_id.as_ref(),
        &[bump_seed],
    ];

    // Validate data
    if buy_price + sell_price != u64::pow(10, betting_pool.decimals as u32) {
        return Err(BettingPoolError::TradePricesIncorrect.into());
    }
    if betting_pool.settled {
        return Err(BettingPoolError::AlreadySettled.into());
    }
    assert_keys_unequal(*buyer_info.key, *seller_info.key)?;
    assert_keys_equal(*long_token_mint_info.owner, spl_token::id())?;
    assert_keys_equal(*short_token_mint_info.owner, spl_token::id())?;
    assert_keys_equal(buyer_long_token_account.owner, *buyer_info.key)?;
    assert_keys_equal(buyer_short_token_account.owner, *buyer_info.key)?;
    assert_keys_equal(seller_long_token_account.owner, *seller_info.key)?;
    assert_keys_equal(seller_short_token_account.owner, *seller_info.key)?;
    assert_keys_equal(buyer_account.owner, *buyer_info.key)?;
    assert_keys_equal(seller_account.owner, *seller_info.key)?;
    assert_keys_equal(authority_key, *authority_info.key)?;
    assert_keys_equal(
        *long_token_mint_info.key,
        betting_pool.long_mint_account_pubkey,
    )?;
    assert_keys_equal(
        *short_token_mint_info.key,
        betting_pool.short_mint_account_pubkey,
    )?;
    assert_keys_equal(*escrow_account_info.key, betting_pool.escrow_account_pubkey)?;
    assert_keys_equal(
        buyer_long_token_account.mint,
        betting_pool.long_mint_account_pubkey,
    )?;
    assert_keys_equal(
        buyer_short_token_account.mint,
        betting_pool.short_mint_account_pubkey,
    )?;
    assert_keys_equal(
        seller_long_token_account.mint,
        betting_pool.long_mint_account_pubkey,
    )?;
    assert_keys_equal(
        seller_short_token_account.mint,
        betting_pool.short_mint_account_pubkey,
    )?;
    assert_keys_equal(buyer_account.mint, betting_pool.escrow_mint_account_pubkey)?;
    assert_keys_equal(seller_account.mint, betting_pool.escrow_mint_account_pubkey)?;

    let n = size;
    let n_b = buyer_short_token_account.amount;
    let n_s = seller_long_token_account.amount;

    let mut b_l = buyer_long_token_account.amount;
    let mut b_s = n_b;
    let mut s_l = n_s;
    let mut s_s = seller_short_token_account.amount;

    match [n_b >= n, n_s >= n] {
        /*
        When n is less than both n_b and n_s, this means that both buyer and seller are simply reducing their existing inventory.
        Therefore, we can just remove n long tokens and n short tokens from circulation. Both parties are also entitled to the locked up
        funds for their positions that were closed. This always results in a decrease in total circulation.
        */
        [true, true] => {
            msg!("Case 1");
            spl_burn(
                &token_program_info,
                &buyer_short_token_account_info,
                &short_token_mint_info,
                &buyer_info,
                n,
            )?;
            spl_burn(
                &token_program_info,
                &seller_long_token_account_info,
                &long_token_mint_info,
                &seller_info,
                n,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &buyer_account_info,
                &authority_info,
                n * sell_price,
                1,
                seeds,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &seller_account_info,
                &authority_info,
                n * buy_price,
                1,
                seeds,
            )?;
            b_s -= n;
            s_l -= n;
            betting_pool.decrement_supply(n)?;
        }
        /*
        When n is greater than both n_b and n_s, this means that both buyer and seller have put on a position that is different from their
        existing position. We will first burn the tokens of representing the opposite position and then mint new tokens to ensure the buyer's
        change is +n and the seller's change is -n. Both parties are also entitled to the locked up funds for their positions that were closed.
        The net change in tokens can be calculated as follows: (-n_b - n_s + 2n - n_b - n_s) / 2 = n - n_b - n_s. If this quantity is positive, this
        means that the trade causes a net increase in the total supply of contracts in the betting pool. Otherwise, it results in a net decrease
        in total circulation.
        */
        [false, false] => {
            msg!("Case 2");
            spl_burn(
                &token_program_info,
                &buyer_short_token_account_info,
                &short_token_mint_info,
                &buyer_info,
                n_b,
            )?;
            spl_burn(
                &token_program_info,
                &seller_long_token_account_info,
                &long_token_mint_info,
                &seller_info,
                n_s,
            )?;
            b_s -= n_b;
            s_l -= n_s;
            spl_mint_to(
                &token_program_info,
                &buyer_long_token_account_info,
                &long_token_mint_info,
                &authority_info,
                n - n_b,
                seeds,
            )?;
            spl_mint_to(
                &token_program_info,
                &seller_short_token_account_info,
                &short_token_mint_info,
                &authority_info,
                n - n_s,
                seeds,
            )?;
            b_l += n - n_b;
            s_s += n - n_s;
            spl_token_transfer(
                &token_program_info,
                &buyer_account_info,
                &escrow_account_info,
                &buyer_info,
                (n - n_b) * buy_price,
            )?;
            spl_token_transfer(
                &token_program_info,
                &seller_account_info,
                &escrow_account_info,
                &seller_info,
                (n - n_s) * sell_price,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &buyer_account_info,
                &authority_info,
                n_b * sell_price,
                1,
                seeds,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &seller_account_info,
                &authority_info,
                n_s * buy_price,
                1,
                seeds,
            )?;
            if n > n_b + n_s {
                betting_pool.increment_supply(n - n_b - n_s);
            } else {
                betting_pool.decrement_supply(n - n_b - n_s)?;
            }
        }
        /*
        When n is greater than n_b bust less than n_s, this means that the buyer has put on a position that is different from their
        existing position, and the seller has reduced their inventory. We will burn and mint tokens such the buyer's net change in
        position is +n and the seller's net change is -n. Both parties are also entitled to the locked up funds for their positions that were closed.
        The net change in tokens can be calculated as follows: (-n - n_s + n - n_s) / 2 = -n_s. This always results in a decrease in total
        circulation.
        */
        [true, false] => {
            msg!("Case 3");
            spl_burn(
                &token_program_info,
                &buyer_short_token_account_info,
                &short_token_mint_info,
                &buyer_info,
                n,
            )?;
            spl_burn(
                &token_program_info,
                &seller_long_token_account_info,
                &long_token_mint_info,
                &seller_info,
                n_s,
            )?;
            b_s -= n;
            s_l -= n_s;
            spl_mint_to(
                &token_program_info,
                &seller_short_token_account_info,
                &short_token_mint_info,
                &authority_info,
                n - n_s,
                seeds,
            )?;
            s_s += n - n_s;
            spl_token_transfer(
                &token_program_info,
                &seller_account_info,
                &escrow_account_info,
                &seller_info,
                (n - n_s) * sell_price,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &seller_account_info,
                &authority_info,
                n_s * buy_price,
                1,
                seeds,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &buyer_account_info,
                &authority_info,
                n * sell_price,
                1,
                seeds,
            )?;
            betting_pool.decrement_supply(n_s)?;
        }
        /*
        When n is greater than n_s bust less than n_b, this means that the seller has put on a position that is different from their
        existing position, and the buyer has reduced their inventory. We will burn and mint tokens such the buyer's net change in
        position is +n and the seller's net change is -n. Both parties are also entitled to the locked up funds for their positions that were closed.
        The net change in tokens can be calculated as follows: (-n - n_b + n - n_b) / 2 = -n_b. This always results in a decrease in total
        circulation.
        */
        [false, true] => {
            msg!("Case 4");
            spl_burn(
                &token_program_info,
                &seller_long_token_account_info,
                &long_token_mint_info,
                &seller_info,
                n,
            )?;
            spl_burn(
                &token_program_info,
                &buyer_short_token_account_info,
                &short_token_mint_info,
                &buyer_info,
                n_b,
            )?;
            b_s -= n_b;
            s_l -= n;
            spl_mint_to(
                &token_program_info,
                &buyer_long_token_account_info,
                &long_token_mint_info,
                &authority_info,
                n - n_b,
                seeds,
            )?;
            b_l += n - n_b;
            spl_token_transfer(
                &token_program_info,
                &buyer_account_info,
                &escrow_account_info,
                &buyer_info,
                (n - n_b) * buy_price,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &buyer_account_info,
                &authority_info,
                n_b * sell_price,
                1,
                seeds,
            )?;
            spl_token_transfer_signed(
                &token_program_info,
                &escrow_account_info,
                &seller_account_info,
                &authority_info,
                n * buy_price,
                1,
                seeds,
            )?;
            betting_pool.decrement_supply(n_b)?;
        }
    }
    // Delegate the burn authority to the PDA, so a private key is unnecessary on collection
    // This can probably be optimized to reduce the number of instructions needed at some point
    spl_approve(
        &token_program_info,
        &buyer_long_token_account_info,
        &long_token_mint_info,
        &authority_info,
        &buyer_info,
        b_l,
        long_token_mint.decimals,
    )?;
    spl_approve(
        &token_program_info,
        &seller_short_token_account_info,
        &short_token_mint_info,
        &authority_info,
        &seller_info,
        s_s,
        short_token_mint.decimals,
    )?;
    spl_approve(
        &token_program_info,
        &buyer_short_token_account_info,
        &short_token_mint_info,
        &authority_info,
        &buyer_info,
        b_s,
        short_token_mint.decimals,
    )?;
    spl_approve(
        &token_program_info,
        &seller_long_token_account_info,
        &long_token_mint_info,
        &authority_info,
        &seller_info,
        s_l,
        long_token_mint.decimals,
    )?;
    betting_pool.serialize(&mut *pool_account_info.data.borrow_mut())?;
    Ok(())
}

pub fn process_settle(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    // This should NEVER be called directly (otherwise this is literally a rug)
    // The `pool_owner_info` needs to approve this action, so the recommended use case is to have a higher
    // level program own the pool and use an oracle to resolve settlements 
    msg!("Settle");
    let account_info_iter = &mut accounts.iter();
    let pool_account_info = next_account_info(account_info_iter)?;
    let winning_mint_account_info = next_account_info(account_info_iter)?;
    let pool_owner_info = next_account_info(account_info_iter)?;

    let mut betting_pool = BettingPool::try_from_slice(&pool_account_info.data.borrow_mut())?;
    if !pool_owner_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if betting_pool.settled {
        return Err(BettingPoolError::AlreadySettled.into());
    }

    assert_keys_equal(*pool_owner_info.key, betting_pool.owner)?;
    if *winning_mint_account_info.key == betting_pool.long_mint_account_pubkey
        || *winning_mint_account_info.key == betting_pool.short_mint_account_pubkey
    {
        betting_pool.winning_side_pubkey = *winning_mint_account_info.key;
    } else {
        return Err(BettingPoolError::InvalidWinner.into());
    }
    betting_pool.settled = true;
    betting_pool.serialize(&mut *pool_account_info.data.borrow_mut())?;
    Ok(())
}

pub fn process_collect(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Collect");
    let account_info_iter = &mut accounts.iter();
    let pool_account_info = next_account_info(account_info_iter)?;
    let collector_info = next_account_info(account_info_iter)?;
    let collector_long_token_account_info = next_account_info(account_info_iter)?;
    let collector_short_token_account_info = next_account_info(account_info_iter)?;
    let collector_account_info = next_account_info(account_info_iter)?;
    let long_token_mint_info = next_account_info(account_info_iter)?;
    let short_token_mint_info = next_account_info(account_info_iter)?;
    let escrow_account_info = next_account_info(account_info_iter)?;
    let escrow_authority_info = next_account_info(account_info_iter)?;
    let fee_payer_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let collector_long_token_account: Account =
        assert_initialized(collector_long_token_account_info)?;
    let collector_short_token_account: Account =
        assert_initialized(collector_short_token_account_info)?;
    let collector_account: Account = assert_initialized(collector_account_info)?;
    let escrow_account: Account = assert_initialized(escrow_account_info)?;
    let mut betting_pool = BettingPool::try_from_slice(&pool_account_info.data.borrow_mut())?;

    // Get program derived address for escrow
    let (escrow_owner_key, bump_seed) = Pubkey::find_program_address(
        &[
            long_token_mint_info.key.as_ref(),
            short_token_mint_info.key.as_ref(),
            token_program_info.key.as_ref(),
            program_id.as_ref(),
        ],
        program_id,
    );
    let seeds = &[
        long_token_mint_info.key.as_ref(),
        short_token_mint_info.key.as_ref(),
        token_program_info.key.as_ref(),
        program_id.as_ref(),
        &[bump_seed],
    ];

    if !betting_pool.settled {
        return Err(BettingPoolError::BetNotSettled.into());
    }
    assert_owned_by(long_token_mint_info, &spl_token::id())?;
    assert_owned_by(short_token_mint_info, &spl_token::id())?;
    assert_keys_equal(collector_long_token_account.owner, *collector_info.key)?;
    assert_keys_equal(collector_short_token_account.owner, *collector_info.key)?;
    assert_keys_equal(collector_account.owner, *collector_info.key)?;
    assert_keys_equal(escrow_owner_key, *escrow_authority_info.key)?;
    assert_keys_equal(
        *long_token_mint_info.key,
        betting_pool.long_mint_account_pubkey,
    )?;
    assert_keys_equal(
        *short_token_mint_info.key,
        betting_pool.short_mint_account_pubkey,
    )?;
    assert_keys_equal(*escrow_account_info.key, betting_pool.escrow_account_pubkey)?;
    assert_keys_equal(*escrow_account_info.key, betting_pool.escrow_account_pubkey)?;
    assert_keys_equal(
        collector_long_token_account.mint,
        betting_pool.long_mint_account_pubkey,
    )?;
    assert_keys_equal(
        collector_short_token_account.mint,
        betting_pool.short_mint_account_pubkey,
    )?;
    assert_keys_equal(
        collector_account.mint,
        betting_pool.escrow_mint_account_pubkey,
    )?;

    let winner_long = collector_long_token_account.mint == betting_pool.winning_side_pubkey;
    let winner_short = collector_short_token_account.mint == betting_pool.winning_side_pubkey;
    let reward = match [winner_long, winner_short] {
        [true, false] => collector_long_token_account.amount,
        [false, true] => collector_short_token_account.amount,
        _ => return Err(BettingPoolError::TokenNotFoundInPool.into()),
    };

    topup(
        escrow_authority_info,
        rent_info,
        system_account_info,
        fee_payer_info,
        1,
    )?;
    spl_burn_signed(
        &token_program_info,
        &collector_long_token_account_info,
        &long_token_mint_info,
        &escrow_authority_info,
        collector_long_token_account.amount,
        seeds,
    )?;
    spl_burn_signed(
        &token_program_info,
        &collector_short_token_account_info,
        &short_token_mint_info,
        &escrow_authority_info,
        collector_short_token_account.amount,
        seeds,
    )?;
    if reward > 0 {
        spl_token_transfer_signed(
            &token_program_info,
            &escrow_account_info,
            &collector_account_info,
            &escrow_authority_info,
            reward * escrow_account.amount,
            betting_pool.circulation,
            seeds,
        )?;
        betting_pool.decrement_supply(reward)?;
    }
    betting_pool.serialize(&mut *pool_account_info.data.borrow_mut())?;
    Ok(())
}