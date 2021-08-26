use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction::create_account,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::state::Account;

use crate::{
    error::GameError,
    processor::{
        security::VerifyType,
        spl_token::{spl_token_init_account, TokenInitializeAccountParams},
        util::account_exists,
    },
    state::{
        GameState, PlayerRoundState, RoundState, StateType, GAME_STATE_SIZE,
        PLAYER_ROUND_STATE_SIZE, ROUND_STATE_SIZE,
    },
};

// --------------------------------------- public

/// Builds seed + verifies + deserializes pda
pub fn deserialize_game_state<'a>(
    game_state_info: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> Result<(GameState, String, u8), ProgramError> {
    let game_state: GameState = GameState::try_from_slice(&game_state_info.data.borrow_mut())?;
    game_state.verify_type()?;
    let game_state_seed = format!("{}{}", GAME_STATE_SEED, game_state.version);
    let game_state_bump =
        verify_pda_matches(game_state_seed.as_bytes(), program_id, game_state_info)?;
    Ok((game_state, game_state_seed, game_state_bump))
}

/// Builds seed + verifies + creates pda
pub fn create_game_state<'a>(
    game_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    version: u64,
    program_id: &Pubkey,
) -> Result<GameState, ProgramError> {
    let game_state_seed = format!("{}{}", GAME_STATE_SEED, version);
    create_pda_with_space(
        game_state_seed.as_bytes(),
        game_state_info,
        GAME_STATE_SIZE,
        program_id,
        funder_info,
        system_program_info,
        program_id,
    )?;
    GameState::try_from_slice(&game_state_info.data.borrow_mut())
        .map_err(|_| GameError::UnpackingFailure.into())
}

/// Builds seed + verifies + deserializes pda
pub fn deserialize_round_state<'a>(
    round_state_info: &AccountInfo<'a>,
    round_id: u64,
    version: u64,
    program_id: &Pubkey,
) -> Result<RoundState, ProgramError> {
    let round_state: RoundState = RoundState::try_from_slice(&round_state_info.data.borrow_mut())?;
    round_state.verify_type()?;
    let round_state_seed = format!("{}{}{}", ROUND_STATE_SEED, round_id, version);
    verify_pda_matches(round_state_seed.as_bytes(), program_id, round_state_info)?;
    Ok(round_state)
}

/// Builds seed + verifies + creates pda
pub fn create_round_state<'a>(
    round_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    round_id: u64,
    version: u64,
    program_id: &Pubkey,
) -> Result<RoundState, ProgramError> {
    let round_state_seed = format!("{}{}{}", ROUND_STATE_SEED, round_id, version);
    create_pda_with_space(
        round_state_seed.as_bytes(),
        round_state_info,
        ROUND_STATE_SIZE,
        program_id,
        funder_info,
        system_program_info,
        program_id,
    )?;
    RoundState::try_from_slice(&round_state_info.data.borrow_mut())
        .map_err(|_| GameError::UnpackingFailure.into())
}

/// Builds seed + verifies + deserializes pda
pub fn deserialize_player_round_state<'a>(
    player_round_state_info: &AccountInfo<'a>,
    player_pk: &Pubkey,
    round_id: u64,
    version: u64,
    program_id: &Pubkey,
) -> Result<PlayerRoundState, ProgramError> {
    let player_round_state: PlayerRoundState =
        PlayerRoundState::try_from_slice(&player_round_state_info.data.borrow_mut())?;
    player_round_state.verify_type()?;
    let player_round_state_seed = format!(
        "{}{}{}{}",
        PLAYER_ROUND_STATE_SEED,      //4
        &player_pk.to_string()[..12], //12 - max seed len 32
        round_id,                     //8
        version                       //8
    );
    verify_pda_matches(
        player_round_state_seed.as_bytes(),
        program_id,
        player_round_state_info,
    )?;
    Ok(player_round_state)
}

/// Builds seed + verifies + deserializes/creates pda if missing
pub fn deserialize_or_create_player_round_state<'a>(
    player_round_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    player_pk: &Pubkey,
    round_id: u64,
    version: u64,
    program_id: &Pubkey,
) -> Result<PlayerRoundState, ProgramError> {
    if account_exists(player_round_state_info) {
        deserialize_player_round_state(
            player_round_state_info,
            player_pk,
            round_id,
            version,
            program_id,
        )
    } else {
        let player_round_state_seed = format!(
            "{}{}{}{}",
            PLAYER_ROUND_STATE_SEED, //4
            //todo is 12 characters secure enough? how long would this take to grind?
            &player_pk.to_string()[..12], //12 - max seed len 32
            round_id,                     //8
            version                       //8
        );
        create_pda_with_space(
            player_round_state_seed.as_bytes(),
            player_round_state_info,
            PLAYER_ROUND_STATE_SIZE,
            program_id,
            funder_info,
            system_program_info,
            program_id,
        )?;
        let mut player_round_state: PlayerRoundState =
            PlayerRoundState::try_from_slice(&player_round_state_info.data.borrow_mut())?;
        //initially set the player's public key and round id
        player_round_state.player_pk = *player_pk;
        player_round_state.round_id = round_id;
        player_round_state.TYPE = StateType::PlayerRoundStateTypeV1;
        Ok(player_round_state)
    }
}

/// Builds seed + verifies + deserializes pda
pub fn deserialize_pot<'a>(
    pot_info: &AccountInfo<'a>,
    game_state_info: &AccountInfo<'a>,
    round_id: u64,
    version: u64,
    program_id: &Pubkey,
) -> Result<Account, ProgramError> {
    let pot = Account::unpack(&pot_info.data.borrow_mut())?;
    if pot.owner != *game_state_info.key {
        return Err(GameError::InvalidOwner.into());
    }
    let pot_seed = format!("{}{}{}", POT_SEED, round_id, version);
    verify_pda_matches(pot_seed.as_bytes(), program_id, pot_info)?;
    Ok(pot)
}

/// Builds seed + verifies + creates pda
pub fn create_pot<'a>(
    pot_info: &AccountInfo<'a>,
    game_state_info: &AccountInfo<'a>,
    funder_info: &AccountInfo<'a>,
    mint_info: &AccountInfo<'a>,
    rent_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    round_id: u64,
    version: u64,
    program_id: &Pubkey,
) -> Result<Account, ProgramError> {
    let pot_seed = format!("{}{}{}", POT_SEED, round_id, version);
    create_pda_with_space(
        pot_seed.as_bytes(),
        pot_info,
        spl_token::state::Account::get_packed_len(),
        &spl_token::id(),
        funder_info,
        system_program_info,
        program_id,
    )?;
    // initialize + give game_state pda "ownership" over it
    spl_token_init_account(TokenInitializeAccountParams {
        account: pot_info.clone(),
        mint: mint_info.clone(),
        owner: game_state_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;
    Account::unpack(&pot_info.data.borrow_mut()).map_err(|_| GameError::UnpackingFailure.into())
}

// --------------------------------------- private

const POT_SEED: &str = "pot";
const GAME_STATE_SEED: &str = "game";
const ROUND_STATE_SEED: &str = "round";
const PLAYER_ROUND_STATE_SEED: &str = "pr";

fn create_pda_with_space<'a>(
    pda_seed: &[u8],
    pda_info: &AccountInfo<'a>,
    space: usize,
    owner: &Pubkey,
    funder_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let bump_seed = verify_pda_matches(pda_seed, program_id, pda_info)?;
    let full_seeds: &[&[_]] = &[pda_seed, &[bump_seed]];

    //create a PDA and allocate space inside of it at the same time
    //can only be done from INSIDE the program
    //based on https://github.com/solana-labs/solana-program-library/blob/7c8e65292a6ebc90de54468c665e30bc590c513a/feature-proposal/program/src/processor.rs#L148-L163
    invoke_signed(
        &create_account(
            &funder_info.key,
            &pda_info.key,
            1.max(Rent::get()?.minimum_balance(space)),
            space as u64,
            owner,
        ),
        &[
            funder_info.clone(),
            pda_info.clone(),
            system_program_info.clone(),
        ],
        &[full_seeds], //this is the part you can't do outside the program
    )?;

    msg!("pda created");
    Ok(bump_seed)
}

fn verify_pda_matches(
    pda_seed: &[u8],
    program_id: &Pubkey,
    pda_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (pda, bump_seed) = Pubkey::find_program_address(&[pda_seed], program_id);
    if pda != *pda_info.key {
        msg!("pda doesnt match: {}, {}", pda, *pda_info.key);
        return Err(GameError::PDAMatchFailure.into());
    }
    Ok(bump_seed)
}
