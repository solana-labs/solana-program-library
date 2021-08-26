use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_token::{
    solana_program::program_pack::Pack,
    state::{Account, Mint},
};

use crate::{
    error::GameError,
    instruction::{GameInstruction, InitGameParams, PurchaseKeysParams, WithdrawParams},
    math::{
        common::{TryAdd, TryCast, TryDiv, TryMul, TrySub},
        curve::keys_received,
    },
    processor::{
        pda::{
            create_game_state, create_pot, create_round_state, deserialize_game_state,
            deserialize_or_create_player_round_state, deserialize_player_round_state,
            deserialize_pot, deserialize_round_state,
        },
        security::{
            verify_account_count, verify_account_ownership, verify_is_signer, verify_rent_exempt,
            verify_round_state, verify_token_program, Owner,
        },
        spl_token::{spl_token_transfer, TokenTransferParams},
        util::{
            account_exists, airdrop_winner, calc_new_delay, calculate_player_f3d_share,
            time_is_out, Empty,
        },
    },
    state::{
        StateType, Team, BEAR_FEE_SPLIT, BEAR_POT_SPLIT, BULL_FEE_SPLIT, BULL_POT_SPLIT,
        SNEK_FEE_SPLIT, SNEK_POT_SPLIT, WHALE_FEE_SPLIT, WHALE_POT_SPLIT,
    },
};

pub struct Processor {}

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let instruction = GameInstruction::try_from_slice(data)?;
        match instruction {
            GameInstruction::InitializeGame(game_params) => {
                msg!("init game");
                Self::process_initialize_game(program_id, accounts, game_params)
            }
            GameInstruction::InitializeRound => {
                msg!("init round");
                Self::process_initialize_round(program_id, accounts)
            }
            GameInstruction::PurchaseKeys(purchase_params) => {
                msg!("purchase keys");
                Self::process_purchase_keys(program_id, accounts, purchase_params)
            }
            GameInstruction::WithdrawSol(withdraw_params) => {
                msg!("withdraw sol");
                Self::process_withdraw_sol(program_id, accounts, withdraw_params)
            }
            GameInstruction::EndRound => {
                msg!("end round");
                Self::process_end_round(program_id, accounts)
            }
            GameInstruction::WithdrawCommunityRewards(withdraw_params) => {
                msg!("withdraw community rewards");
                Self::process_community_withdrawal(program_id, accounts, withdraw_params)
            }
            GameInstruction::WithdrawP3DRewards(withdraw_params) => {
                msg!("withdraw p3d rewards");
                Self::process_p3d_withdrawal(program_id, accounts, withdraw_params)
            }
        }
    }

    pub fn process_initialize_game(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        game_params: InitGameParams,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let game_creator_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let com_wallet_info = next_account_info(account_info_iter)?;
        let p3d_wallet_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let expected_owners = [
            //todo is it reasonable to assume that the game creator's account is owned by sys prog?
            Owner::SystemProgram,
            Owner::SystemProgram,
            Owner::TokenProgram,
            Owner::TokenProgram,
            Owner::TokenProgram,
            Owner::NativeLoader,
        ];
        verify_account_ownership(accounts, &expected_owners)?;
        verify_account_count(accounts, 6, 6)?;
        verify_is_signer(game_creator_info)?;
        verify_rent_exempt(&[com_wallet_info, p3d_wallet_info, mint_info])?;

        let InitGameParams {
            version,
            round_init_time,
            round_inc_time_per_key,
            round_max_time,
        } = game_params;

        if account_exists(game_state_info) {
            return Err(GameError::AlreadyInitialized.into());
        }

        Mint::unpack(&mint_info.data.borrow_mut())?; //this proves it's indeed a mint account
        let com_wallet = Account::unpack(&com_wallet_info.data.borrow_mut())?;
        let p3d_wallet = Account::unpack(&p3d_wallet_info.data.borrow_mut())?;
        if com_wallet.mint != *mint_info.key {
            return Err(GameError::MintMatchFailure.into());
        }
        if p3d_wallet.mint != *mint_info.key {
            return Err(GameError::MintMatchFailure.into());
        }

        let mut game_state = create_game_state(
            game_state_info,
            game_creator_info,
            system_program_info,
            version,
            program_id,
        )?;

        game_state.round_id = 0; //will be incremented to 1 when 1st round initialized
        game_state.round_init_time = round_init_time;
        game_state.round_inc_time_per_key = round_inc_time_per_key;
        game_state.round_max_time = round_max_time;
        game_state.version = version;
        game_state.mint = *mint_info.key;
        game_state.game_creator = *game_creator_info.key;
        game_state.community_wallet = *com_wallet_info.key;
        game_state.p3d_wallet = *p3d_wallet_info.key;
        game_state.TYPE = StateType::GameStateTypeV1;
        game_state.serialize(&mut *game_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_initialize_round(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter().peekable();
        let funder_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let mut expected_owners = vec![
            Owner::SystemProgram,
            Owner::Other(*program_id),
            Owner::SystemProgram,
            Owner::SystemProgram,
            Owner::TokenProgram,
            Owner::Sysvar,
            Owner::NativeLoader,
            Owner::BPFLoader,
        ];
        if account_info_iter.peek().is_some() {
            expected_owners.push(Owner::Other(*program_id));
            expected_owners.push(Owner::TokenProgram);
        }
        verify_account_ownership(accounts, &expected_owners)?;
        verify_account_count(accounts, 8, 10)?;
        verify_token_program(token_program_info)?;
        verify_rent_exempt(&[game_state_info, mint_info])?;

        if account_exists(round_state_info) {
            return Err(GameError::AlreadyInitialized.into());
        }

        let (mut game_state, game_state_seed, game_state_bump) =
            deserialize_game_state(game_state_info, program_id)?;
        let previous_round_id = game_state.round_id;
        game_state.round_id.try_self_add(1)?;

        let mut round_state = create_round_state(
            round_state_info,
            funder_info,
            system_program_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        create_pot(
            pot_info,
            game_state_info,
            funder_info,
            mint_info,
            rent_info,
            system_program_info,
            token_program_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;

        // --------------------------------------- deal with previous round
        if previous_round_id != 0 {
            //optional accounts in case this isn't the 1st round
            let previous_round_state_info = next_account_info(account_info_iter)?;
            let previous_round_pot_info = next_account_info(account_info_iter)?;
            let mut previous_round_state = deserialize_round_state(
                previous_round_state_info,
                previous_round_id,
                game_state.version,
                program_id,
            )?;
            deserialize_pot(
                previous_round_pot_info,
                game_state_info,
                previous_round_id,
                game_state.version,
                program_id,
            )?;
            //previous round must have ended, otherwise no-go
            if !previous_round_state.ended {
                return Err(GameError::NotYetEnded.into());
            }
            //move tokens from previous round to current
            let move_over_amount = previous_round_state
                .accum_next_round_share
                .try_sub(previous_round_state.withdrawn_next_round)?;

            spl_token_transfer(TokenTransferParams {
                source: previous_round_pot_info.clone(),
                destination: pot_info.clone(),
                amount: move_over_amount.try_cast()?,
                authority: game_state_info.clone(),
                authority_signer_seeds: &[game_state_seed.as_bytes(), &[game_state_bump]],
                token_program: token_program_info.clone(),
            })?;
            //update current round state & verify amount matches what's in pot
            round_state.accum_sol_pot.try_self_add(move_over_amount)?;
            let pot_after_transfer = deserialize_pot(
                pot_info,
                game_state_info,
                game_state.round_id,
                game_state.version,
                program_id,
            )?;
            assert_eq!(pot_after_transfer.amount as u128, round_state.accum_sol_pot);
            //update previous round state
            previous_round_state
                .withdrawn_next_round
                .try_self_add(move_over_amount)?;
            previous_round_state.serialize(&mut *previous_round_state_info.data.borrow_mut())?;
        }

        // --------------------------------------- update current round state
        let clock = Clock::get()?;
        // all attributes not mentioned automatically start at 0.
        round_state.round_id = game_state.round_id;
        round_state.start_time = clock.unix_timestamp;
        //to calculate end time add the initial time window specified during game initialization
        round_state.end_time = round_state.start_time.try_add(game_state.round_init_time)?;
        round_state.ended = false;
        round_state.TYPE = StateType::RoundStateTypeV1;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;
        game_state.serialize(&mut *game_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_purchase_keys(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        purchase_params: PurchaseKeysParams,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter().peekable();
        let player_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let player_round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let player_token_acc_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let mut affiliate_round_state_info = None;
        let mut affiliate_owner_info = None;
        let mut expected_owners = vec![
            Owner::SystemProgram,
            Owner::Other(*program_id),
            Owner::Other(*program_id),
            Owner::Other(*program_id),
            Owner::TokenProgram,
            Owner::TokenProgram,
            Owner::NativeLoader,
            Owner::BPFLoader,
        ];
        //change the owner if not yet initialized
        if !account_exists(player_round_state_info) {
            expected_owners[3] = Owner::SystemProgram;
        }
        if account_info_iter.peek().is_some() {
            //retrieve the accounts
            affiliate_round_state_info = Some(next_account_info(account_info_iter)?);
            affiliate_owner_info = Some(next_account_info(account_info_iter)?);
            //push the expected owners
            expected_owners.push(Owner::Other(*program_id));
            expected_owners.push(Owner::SystemProgram);
            //change the owner if not yet initialized
            if !account_exists(affiliate_round_state_info.unwrap()) {
                expected_owners[8] = Owner::SystemProgram;
            }
        }
        verify_account_ownership(accounts, &expected_owners)?;
        verify_account_count(accounts, 8, 10)?;
        verify_is_signer(player_info)?;
        verify_token_program(token_program_info)?;
        verify_rent_exempt(&[
            game_state_info,
            round_state_info,
            pot_info,
            player_token_acc_info,
        ])?;

        let PurchaseKeysParams {
            mut sol_to_be_added,
            team,
        } = purchase_params;
        let player_pk = player_info.key;

        let (game_state, _, _) = deserialize_game_state(game_state_info, program_id)?;
        let mut round_state = deserialize_round_state(
            round_state_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        //ensure the round hasn't ended yet
        if time_is_out(&round_state)? {
            return Err(GameError::AlreadyEnded.into());
        }
        deserialize_pot(
            pot_info,
            game_state_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        let mut player_round_state = deserialize_or_create_player_round_state(
            player_round_state_info,
            player_info,
            system_program_info,
            player_pk,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;
        //this is not strictly necessary, but won't hurt
        let player_token_acc = Account::unpack(&player_token_acc_info.data.borrow())?;
        if player_token_acc.owner != *player_info.key {
            return Err(GameError::InvalidOwner.into());
        }
        //no need to verify mint - the transfer below will simply fail if player acc's mint != pot mint

        // --------------------------------------- calc variables
        // if total pot < 100 sol, each user only allowed to contribute 1 sol total
        sol_to_be_added = if round_state.accum_sol_pot < 100.try_mul(LAMPORTS_PER_SOL as u128)?
            && player_round_state
                .accum_sol_added
                .try_add(sol_to_be_added)?
                > LAMPORTS_PER_SOL as u128
        {
            (LAMPORTS_PER_SOL as u128).try_sub(player_round_state.accum_sol_added)?
        } else {
            sol_to_be_added
        };

        let fee_split;
        let player_team = match team {
            0 => {
                round_state
                    .accum_sol_by_team
                    .whale
                    .try_self_add(sol_to_be_added)?;
                fee_split = WHALE_FEE_SPLIT;
                Team::Whale
            }
            1 => {
                round_state
                    .accum_sol_by_team
                    .bear
                    .try_self_add(sol_to_be_added)?;
                fee_split = BEAR_FEE_SPLIT;
                Team::Bear
            }
            3 => {
                round_state
                    .accum_sol_by_team
                    .bull
                    .try_self_add(sol_to_be_added)?;
                fee_split = BULL_FEE_SPLIT;
                Team::Bull
            }
            _ => {
                round_state
                    .accum_sol_by_team
                    .snek
                    .try_self_add(sol_to_be_added)?;
                fee_split = SNEK_FEE_SPLIT;
                Team::Snek
            }
        };
        let pot_percent = 86
            .try_sub(fee_split.f3d as u128)?
            .try_sub(fee_split.p3d as u128)?;

        // Ensure enough lamports are sent to buy at least 1 whole key.
        // In the original game on Ethereum it was possible to purchase <1 key.
        // On Solana however, due to restrictions around doing U256 math, we set the min as 1 key.
        // In practice this means a min participation ticket of:
        //  - 75_000 lamports/key at the beginning of the round (when keys are cheap)
        //  - 1.7 sol/per at max capacity of the game (10bn SOL total - not actually achievable)
        let new_keys = keys_received(round_state.accum_sol_pot, sol_to_be_added)?;
        if new_keys < 1 {
            return Err(GameError::BelowFloor.into());
        }

        // --------------------------------------- transfer funds to pot
        spl_token_transfer(TokenTransferParams {
            source: player_token_acc_info.clone(),
            destination: pot_info.clone(),
            authority: player_info.clone(), //this also enforces player_info to be a signer
            token_program: token_program_info.clone(),
            amount: sol_to_be_added.try_cast()?,
            authority_signer_seeds: &[],
        })?;

        // --------------------------------------- play in airdrop lottery
        //if they deposited > 0.1 sol, they're eligible for airdrop
        if sol_to_be_added > (LAMPORTS_PER_SOL as u128).try_floor_div(10)? {
            let clock = Clock::get()?;

            //with every extra player chance of airdrop increases by 0.1%
            round_state.airdrop_tracker.try_self_add(1)?;

            if airdrop_winner(player_pk, &clock, round_state.airdrop_tracker)? {
                //NOTE: affiliate winnings _exclude_ contribution from this purchase, which is recorded below
                let airdrop_to_distribute = round_state.accum_airdrop_share;
                //3 tiers exist for airdrop
                let prize = if sol_to_be_added > (LAMPORTS_PER_SOL as u128).try_mul(10)? {
                    //10+ sol - win 75% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(75)?.try_floor_div(100)?
                } else if sol_to_be_added > LAMPORTS_PER_SOL as u128 {
                    //1-10 sol - win 50% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(50)?.try_floor_div(100)?
                } else {
                    //0.1-1 sol - win 25% of the accumulated airdrop pot
                    airdrop_to_distribute.try_mul(25)?.try_floor_div(100)?
                };

                //send money
                round_state.accum_airdrop_share.try_self_sub(prize)?;
                player_round_state.accum_winnings.try_self_add(prize)?;
                //reset the lottery
                round_state.airdrop_tracker = 0;
            }
        }

        // --------------------------------------- calc shares
        //2% to community
        let community_share = sol_to_be_added.try_floor_div(50)?;
        //1% to future airdrops
        let airdrop_share = sol_to_be_added.try_floor_div(100)?;
        //1% to next round's pot
        let next_round_share = sol_to_be_added.try_floor_div(100)?;
        //10% to affiliate
        let mut affiliate_share = sol_to_be_added.try_floor_div(10)?;

        let mut p3d_share = 0;
        let mut f3d_share = 0;

        //if player has an affiliate listed, they MUST pass another account
        if player_round_state.has_affiliate_listed() && affiliate_round_state_info.is_none() {
            return Err(GameError::MissingAccount.into());
        }

        //however there is a case where they don't have an affiliate but want to add one -
        //this is why we do the check again
        if affiliate_round_state_info.is_some() && affiliate_owner_info.is_some() {
            //doesn't matter if this the old or the new affiliate. It's the one that will be credited
            //and listed on player's profile (below)
            let mut affiliate_round_state = deserialize_or_create_player_round_state(
                affiliate_round_state_info.unwrap(),
                player_info,
                system_program_info,
                affiliate_owner_info.unwrap().key,
                game_state.round_id,
                game_state.version,
                program_id,
            )?;
            affiliate_round_state
                .accum_aff
                .try_self_add(affiliate_share)?;
            affiliate_round_state
                .serialize(&mut *affiliate_round_state_info.unwrap().data.borrow_mut())?;
            //update the affiliate key going forward (may or may not have changed)
            player_round_state.last_affiliate_pk = *affiliate_owner_info.unwrap().key;
        } else {
            p3d_share.try_self_add(affiliate_share)?;
            affiliate_share = 0;
        }

        p3d_share.try_self_add(
            sol_to_be_added
                .try_mul(fee_split.p3d as u128)?
                .try_floor_div(100)?,
        )?;
        f3d_share.try_self_add(
            sol_to_be_added
                .try_mul(fee_split.f3d as u128)?
                .try_floor_div(100)?,
        )?;

        let still_in_play = sol_to_be_added
            .try_sub(community_share)?
            .try_sub(airdrop_share)?
            .try_sub(next_round_share)?
            .try_sub(affiliate_share)?
            .try_sub(p3d_share)?
            .try_sub(f3d_share)?;
        assert!(still_in_play >= sol_to_be_added.try_mul(pot_percent)?.try_floor_div(100)?);

        // --------------------------------------- serialize round state
        //update leader
        round_state.lead_player_pk = *player_pk;
        round_state.lead_player_team = player_team;
        //update timer
        round_state
            .end_time
            .try_self_add(calc_new_delay(new_keys, &game_state)? as i64)?;
        //update totals
        round_state.accum_keys.try_self_add(new_keys)?;
        round_state.accum_sol_pot.try_self_add(sol_to_be_added)?;
        //distribute shares
        round_state
            .accum_community_share
            .try_self_add(community_share)?;
        round_state
            .accum_airdrop_share
            .try_self_add(airdrop_share)?;
        round_state
            .accum_next_round_share
            .try_self_add(next_round_share)?;
        round_state.accum_aff_share.try_self_add(affiliate_share)?;
        round_state.accum_p3d_share.try_self_add(p3d_share)?;
        round_state.accum_f3d_share.try_self_add(f3d_share)?;
        round_state.still_in_play.try_self_add(still_in_play)?;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

        verify_round_state(&round_state)?;

        // --------------------------------------- serialize player-round state
        //update totals
        player_round_state.accum_keys.try_self_add(new_keys)?;
        player_round_state
            .accum_sol_added
            .try_self_add(sol_to_be_added)?;
        player_round_state.serialize(&mut *player_round_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        withdraw_params: WithdrawParams,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let player_info = next_account_info(account_info_iter)?;
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let player_round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let player_token_acc_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let expected_owners = [
            Owner::SystemProgram,
            Owner::Other(*program_id),
            Owner::Other(*program_id),
            Owner::Other(*program_id),
            Owner::TokenProgram,
            Owner::TokenProgram,
            Owner::NativeLoader,
            Owner::BPFLoader,
        ];
        verify_account_ownership(accounts, &expected_owners)?;
        verify_account_count(accounts, 8, 8)?;
        verify_is_signer(player_info)?;
        verify_token_program(token_program_info)?;
        verify_rent_exempt(&[
            game_state_info,
            round_state_info,
            player_round_state_info,
            pot_info,
            player_token_acc_info,
        ])?;

        let WithdrawParams { withdraw_for_round } = withdraw_params;

        let (game_state, game_state_seed, game_state_bump) =
            deserialize_game_state(game_state_info, program_id)?;
        let round_state = deserialize_round_state(
            round_state_info,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        deserialize_pot(
            pot_info,
            game_state_info,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        let mut player_round_state = deserialize_or_create_player_round_state(
            player_round_state_info,
            player_info,
            system_program_info,
            player_info.key,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        //verify the destination token account actually belongs to the player
        let player_token_acc = Account::unpack(&player_token_acc_info.data.borrow())?;
        if player_token_acc.owner != *player_info.key {
            return Err(GameError::InvalidOwner.into());
        }
        //no need to verify mint - the transfer below will simply fail if player acc's mint != pot mint

        // --------------------------------------- calc withdrawal amounts
        // No, you don't need to wait for round end to withdraw winnings.
        // Grand prize will not have been added yet,
        // and airdrop lottery winnings should be available to user to withdraw.
        let winnings_to_withdraw = player_round_state
            .accum_winnings
            .try_sub(player_round_state.withdrawn_winnings)?;
        let aff_to_withdraw = player_round_state
            .accum_aff
            .try_sub(player_round_state.withdrawn_aff)?;
        let f3d_to_withdraw = calculate_player_f3d_share(
            player_round_state.accum_keys,
            round_state.accum_keys,
            round_state.accum_f3d_share,
        )?
        .try_sub(player_round_state.withdrawn_f3d)?;
        let total_to_withdraw = winnings_to_withdraw
            .try_add(aff_to_withdraw)?
            .try_add(f3d_to_withdraw)?;

        // --------------------------------------- transfer tokens
        if total_to_withdraw == 0 {
            return Ok(());
        }
        spl_token_transfer(TokenTransferParams {
            source: pot_info.clone(),
            destination: player_token_acc_info.clone(),
            amount: total_to_withdraw.try_cast()?,
            authority: game_state_info.clone(),
            authority_signer_seeds: &[game_state_seed.as_bytes(), &[game_state_bump]],
            token_program: token_program_info.clone(),
        })?;

        // --------------------------------------- update player state
        player_round_state
            .withdrawn_aff
            .try_self_add(aff_to_withdraw)?;
        player_round_state
            .withdrawn_winnings
            .try_self_add(winnings_to_withdraw)?;
        player_round_state
            .withdrawn_f3d
            .try_self_add(f3d_to_withdraw)?;
        player_round_state.serialize(&mut *player_round_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_end_round(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let winner_state_info = next_account_info(account_info_iter)?;
        let expected_owners = [
            Owner::Other(*program_id),
            Owner::Other(*program_id),
            Owner::Other(*program_id),
        ];
        verify_account_ownership(accounts, &expected_owners)?;
        verify_account_count(accounts, 3, 3)?;
        verify_rent_exempt(&[game_state_info, round_state_info, winner_state_info])?;

        let (game_state, _, _) = deserialize_game_state(game_state_info, program_id)?;
        let mut round_state = deserialize_round_state(
            round_state_info,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;

        if !time_is_out(&round_state)? {
            return Err(GameError::NotYetEnded.into());
        }

        let mut player_round_state = deserialize_player_round_state(
            winner_state_info,
            &round_state.lead_player_pk,
            game_state.round_id,
            game_state.version,
            program_id,
        )?;

        // --------------------------------------- calc shares
        let to_be_divided = round_state.still_in_play;
        if to_be_divided == 0 || round_state.lead_player_pk.is_empty() {
            return Ok(());
        }

        let pot_split = match round_state.lead_player_team {
            Team::Whale => WHALE_POT_SPLIT,
            Team::Bear => BEAR_POT_SPLIT,
            Team::Snek => SNEK_POT_SPLIT,
            Team::Bull => BULL_POT_SPLIT,
        };
        let next_round_percent = 50
            .try_sub(pot_split.p3d as u128)?
            .try_sub(pot_split.f3d as u128)?;

        //2% to community
        let community_share = to_be_divided.try_floor_div(50)?;

        //p3d/f3d/next round according to team (always adds up to 50%)
        let p3d_share = to_be_divided
            .try_mul(pot_split.p3d as u128)?
            .try_floor_div(100)?;
        let f3d_share = to_be_divided
            .try_mul(pot_split.f3d as u128)?
            .try_floor_div(100)?;
        let next_round_share = to_be_divided
            .try_mul(next_round_percent)?
            .try_floor_div(100)?;

        //remaining 48% + dust to winner
        let grand_prize = to_be_divided
            .try_sub(community_share)?
            .try_sub(f3d_share)?
            .try_sub(p3d_share)?
            .try_sub(next_round_share)?;
        assert!(grand_prize >= to_be_divided.try_mul(48)?.try_floor_div(100)?);

        // --------------------------------------- assign funds to winner
        player_round_state
            .accum_winnings
            .try_self_add(grand_prize)?;
        player_round_state.serialize(&mut *winner_state_info.data.borrow_mut())?;

        // --------------------------------------- update round state
        round_state.ended = true;
        //update shares
        round_state
            .accum_community_share
            .try_self_add(community_share)?;
        round_state
            .accum_next_round_share
            .try_self_add(next_round_share)?;
        round_state.accum_p3d_share.try_self_add(p3d_share)?;
        round_state.accum_f3d_share.try_self_add(f3d_share)?;
        round_state.final_prize_share.try_self_add(grand_prize)?;
        round_state.still_in_play = 0;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

        verify_round_state(&round_state)?;

        Ok(())
    }

    pub fn process_community_withdrawal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        withdraw_params: WithdrawParams,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let game_state_info = next_account_info(account_info_iter)?;
        let round_state_info = next_account_info(account_info_iter)?;
        let pot_info = next_account_info(account_info_iter)?;
        let com_wallet_info = next_account_info(account_info_iter)?;
        let com_wallet_owner_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let expected_owners = [
            Owner::Other(*program_id),
            Owner::Other(*program_id),
            Owner::TokenProgram,
            Owner::TokenProgram,
            Owner::SystemProgram,
            Owner::BPFLoader,
        ];
        verify_account_ownership(accounts, &expected_owners)?;
        verify_account_count(accounts, 6, 6)?;
        verify_is_signer(com_wallet_owner_info)?;
        verify_token_program(token_program_info)?;
        verify_rent_exempt(&[game_state_info, round_state_info, pot_info, com_wallet_info])?;

        let WithdrawParams { withdraw_for_round } = withdraw_params;

        let (game_state, game_state_seed, game_state_bump) =
            deserialize_game_state(game_state_info, program_id)?;
        let mut round_state = deserialize_round_state(
            round_state_info,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        deserialize_pot(
            pot_info,
            game_state_info,
            withdraw_for_round,
            game_state.version,
            program_id,
        )?;
        //ensure the right community wallet is passed
        if game_state.community_wallet != *com_wallet_info.key {
            return Err(GameError::WrongAccount.into());
        }
        //ensure tx comes from community wallet's owner
        let com_wallet = Account::unpack(&com_wallet_info.data.borrow_mut())?;
        if com_wallet.owner != *com_wallet_owner_info.key {
            return Err(GameError::InvalidOwner.into());
        }

        // --------------------------------------- transfer tokens
        let amount_to_withdraw = round_state
            .accum_community_share
            .try_sub(round_state.withdrawn_com)?;
        if amount_to_withdraw == 0 {
            return Ok(());
        }
        spl_token_transfer(TokenTransferParams {
            source: pot_info.clone(),
            destination: com_wallet_info.clone(),
            amount: amount_to_withdraw.try_cast()?,
            authority: game_state_info.clone(),
            authority_signer_seeds: &[game_state_seed.as_bytes(), &[game_state_bump]],
            token_program: token_program_info.clone(),
        })?;

        // --------------------------------------- update round state
        round_state.withdrawn_com.try_self_add(amount_to_withdraw)?;
        round_state.serialize(&mut *round_state_info.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_p3d_withdrawal(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo],
        _withdraw_params: WithdrawParams,
    ) -> ProgramResult {
        unimplemented!(
            "This basically follows the community withdrawal one for one, \
                        with the only 2 differences being: \
                        1) the destination account passed (p3d instead of com),\
                        2) the round state amounts updated (p3d instead of com)\
                        \
                        Since the app is for demo purposes, decided not to duplicate code."
        )
    }
}
