use std::str::FromStr;

use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::GameError,
    math::common::{TryDiv, TryMul},
    processor::rng::pseudo_rng,
    state::{GameState, RoundState},
};

/// The original math for this is unnecessary convoluted and we decided to ignore it.
/// Ultimately this comes down to a simple equation: (player's keys / total keys) * total f3d earnings.
/// That's the approach taken below. For anyone interested in original math follow these links:
/// https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L1533
/// https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L1125
pub fn calculate_player_f3d_share(
    player_keys: u128,
    total_keys: u128,
    accum_f3d: u128,
) -> Result<u128, ProgramError> {
    //in theory, there might be unaccounted dust left here.
    //eg player1 keys = 333, player2 keys =  total keys = 1000, f3t pot = 100
    //then player1 will get 33, player2 will get 66, and 1 will be left as dust
    //in practice, however, to account for it would have to coordinate all withdrawals by all players
    //which of course isn't possible. So it will just be left in the protocol
    player_keys.try_mul(accum_f3d)?.try_floor_div(total_keys)
}

pub fn airdrop_winner(
    player_pk: &Pubkey,
    clock: &Clock,
    airdrop_tracker: u64,
) -> Result<bool, ProgramError> {
    let lottery_ticket = pseudo_rng(player_pk, clock)?;
    Ok(lottery_ticket < airdrop_tracker as u128)
}

pub fn account_exists(acc: &AccountInfo) -> bool {
    let does_not_exist = **acc.lamports.borrow() == 0 || acc.data_is_empty();
    !does_not_exist
}

pub fn is_zero(buf: &[u8]) -> bool {
    let (prefix, aligned, suffix) = unsafe { buf.align_to::<u128>() };

    prefix.iter().all(|&x| x == 0)
        && suffix.iter().all(|&x| x == 0)
        && aligned.iter().all(|&x| x == 0)
}

pub trait Empty {
    fn is_empty(&self) -> bool;
}
impl Empty for Pubkey {
    fn is_empty(&self) -> bool {
        is_zero(&self.to_bytes()[..])
    }
}

pub fn time_is_out(round_state: &RoundState) -> Result<bool, ProgramError> {
    let clock = Clock::get()?;
    msg!(
        "round time left (s): {}",
        round_state.end_time - clock.unix_timestamp
    );
    Ok(round_state.end_time < clock.unix_timestamp)
}

/// New added delay = minimum of:
/// - number of keys purchased * time per key
/// - 24h from now
pub fn calc_new_delay(new_keys: u128, game_state: &GameState) -> Result<u128, ProgramError> {
    let delay_based_on_keys = new_keys.try_mul(game_state.round_inc_time_per_key as u128)?;
    Ok(delay_based_on_keys.min(game_state.round_max_time as u128))
}

pub fn load_pk(addr: &str) -> Result<Pubkey, ProgramError> {
    Pubkey::from_str(addr).map_err(|_| GameError::WrongAccount.into())
}
