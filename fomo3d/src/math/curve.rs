use solana_program::{msg, native_token::LAMPORTS_PER_SOL, program_error::ProgramError};

use crate::{
    error::GameError,
    math::common::{TryAdd, TryDiv, TryMul, TryPow, TrySqrt, TrySub},
};

pub fn keys_received(current_sol: u128, new_sol: u128) -> Result<u128, ProgramError> {
    if current_sol == 0 && new_sol == 0 {
        return Ok(0);
    }
    let total_keys = sol_to_keys(current_sol.try_add(new_sol)?)?;
    let current_keys = sol_to_keys(current_sol)?;
    total_keys.try_sub(current_keys)
}

pub fn sol_received(current_keys: u128, sold_keys: u128) -> Result<u128, ProgramError> {
    if current_keys == 0 && sold_keys == 0 {
        return Ok(0);
    }
    let total_sol = keys_to_sol(current_keys)?;
    let remaining_sol = keys_to_sol(current_keys.try_sub(sold_keys)?)?;
    total_sol.try_sub(remaining_sol)
}

//constants from https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717
//had to adjust to make them fit SOL instead of ETH - basically divided the numbers to make them smaller
//the curve is exactly the same, no information has been lost
const A: u128 = 10000000;
const B: u128 = 3125000;
const C: u128 = 562498828125610000000000;
const D: u128 = 749999218750;
const E: u128 = 1562500;

/// (!) acceptable input range:
///     min: 75_001 lamports -> 1 key
///     max: 10bn sol -> 11_313_228_509 keys (unreachable - solana only has a max supply of 500m)
fn sol_to_keys(sol: u128) -> Result<u128, ProgramError> {
    if sol == 0 {
        return Ok(0);
    } else if sol > 10_000_000_000.try_mul(LAMPORTS_PER_SOL as u128)? {
        return Err(GameError::AboveThreshold.into());
    }
    // (sqrt[(sol * a * b) + c] - d) / e
    sol.try_mul(A)?
        .try_mul(B)?
        .try_add(C)?
        .try_sqrt()?
        .try_sub(D)?
        .try_floor_div(E)
}

/// (!) acceptable input range:
///     min: 1 key -> 75_000 lamports
///     max: 11_313_228_509 keys -> 9_999_999_998_820_763_638 lamports
fn keys_to_sol(keys: u128) -> Result<u128, ProgramError> {
    if keys > 11313228509 {
        msg!("passed in keys amount of {} exceeds max threshold", keys);
        return Err(GameError::AboveThreshold.into());
    }
    // [(ke + d)^2 - c] / ab
    keys.try_mul(E)?
        .try_add(D)?
        .try_pow(2)?
        .try_sub(C)?
        .try_floor_div(A.try_mul(B)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keys_received() {
        // --------------------------------------- lower bound
        let current_sol = 0;
        let new_sol = 0;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 0);
        // --------------------------------------- the min
        let current_sol = 0;
        let new_sol = 75000;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 0);
        let current_sol = 0;
        let new_sol = 75001;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 1);
        // --------------------------------------- the bulk
        //initially (when total key pool is small) keys are cheap
        let current_sol = 0 * LAMPORTS_PER_SOL as u128;
        let new_sol = 1 * LAMPORTS_PER_SOL as u128;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 13153);
        //later (as pool grows) keys become more expensive
        let current_sol = 100 * LAMPORTS_PER_SOL as u128;
        let new_sol = 1 * LAMPORTS_PER_SOL as u128;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 5197);
        let current_sol = 10000 * LAMPORTS_PER_SOL as u128;
        let new_sol = 1 * LAMPORTS_PER_SOL as u128;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 565);
        // --------------------------------------- the max
        // effectively buying the last most expensive key
        let current_sol = 10 * LAMPORTS_PER_SOL as u128 * LAMPORTS_PER_SOL as u128 - 1_767_766_955;
        let new_sol = 1_767_766_955;
        let new_keys = keys_received(current_sol, new_sol).unwrap();
        assert_eq!(new_keys, 1);
        // --------------------------------------- upper bound
        let current_sol = 10 * LAMPORTS_PER_SOL as u128 * LAMPORTS_PER_SOL as u128 - 1_767_766_955;
        let new_sol = 1_767_766_955 + 1;
        let new_keys = keys_received(current_sol, new_sol);
        assert!(new_keys.is_err());
    }

    #[test]
    fn test_sol_received() {
        // --------------------------------------- lower bound
        let current_keys = 0;
        let sold_keys = 0;
        let earned_sol = sol_received(current_keys, sold_keys).unwrap();
        //can't divide by 0, so checking earned_sol instead
        assert_eq!(earned_sol, 0);
        // --------------------------------------- the min
        let current_keys = 1; //if we're selling keys we must have at least that many
        let sold_keys = 1;
        let earned_sol = sol_received(current_keys, sold_keys).unwrap();
        let sol_per_key = earned_sol.try_floor_div(sold_keys).unwrap();
        assert_eq!(sol_per_key, 75000);
        // --------------------------------------- the bulk
        //initially (when pool is small), keys are cheap
        let current_keys = 1000;
        let sold_keys = 100;
        let earned_sol = sol_received(current_keys, sold_keys).unwrap();
        let sol_per_key = earned_sol.try_floor_div(sold_keys).unwrap();
        assert_eq!(sol_per_key, 75148);
        //later (as pool grows), keys become exponentially more expensive
        let current_keys = 100_000;
        let sold_keys = 10_000;
        let earned_sol = sol_received(current_keys, sold_keys).unwrap();
        let sol_per_key = earned_sol.try_floor_div(sold_keys).unwrap();
        assert_eq!(sol_per_key, 89843);
        let current_keys = 100_000_000;
        let sold_keys = 10_000_000;
        let earned_sol = sol_received(current_keys, sold_keys).unwrap();
        let sol_per_key = earned_sol.try_floor_div(sold_keys).unwrap();
        assert_eq!(sol_per_key, 14918749);
        // --------------------------------------- the max
        //calc how much 1 key costs at the end of the game
        let current_keys = 11313228509 - 1;
        let sold_keys = 1;
        let earned_sol = sol_received(current_keys, sold_keys).unwrap();
        let sol_per_key = earned_sol.try_floor_div(sold_keys).unwrap();
        assert_eq!(sol_per_key, 1_767_766_955);
    }

    #[test]
    fn test_keys_to_sol() {
        // --------------------------------------- lower bound
        let keys = 0;
        let lamp = keys_to_sol(keys).unwrap();
        assert_eq!(lamp, 0);
        // --------------------------------------- the min
        let keys = 1;
        let lamp = keys_to_sol(keys).unwrap();
        assert_eq!(lamp, 75000);
        // --------------------------------------- the bulk
        let keys = 100;
        let lamp = keys_to_sol(keys).unwrap();
        assert_eq!(lamp, 7500773);
        let keys = 10000;
        let lamp = keys_to_sol(keys).unwrap();
        assert_eq!(lamp, 757811718);
        let keys = 1000000;
        let lamp = keys_to_sol(keys).unwrap();
        assert_eq!(lamp, 153124921875);
        // --------------------------------------- the max
        let keys = 11313228509;
        let lamp = keys_to_sol(keys).unwrap();
        assert_eq!(lamp, 9999999998820763638);
        // --------------------------------------- upper bound
        let keys = 11313228509 + 1;
        let lamp = keys_to_sol(keys);
        assert!(lamp.is_err());
    }

    #[test]
    fn test_sol_to_keys() {
        // --------------------------------------- lower bound
        let lamp = 0;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 0);
        let lamp = 1;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 0);
        // --------------------------------------- the min
        let lamp = 75001;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 1);
        // --------------------------------------- the bulk
        //1 sol
        let lamp = (1 * LAMPORTS_PER_SOL) as u128;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 13153);
        let lamp = (10 * LAMPORTS_PER_SOL) as u128;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 118665);
        let lamp = (100 * LAMPORTS_PER_SOL) as u128;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 748983);
        //1 bn sol
        let lamp = (LAMPORTS_PER_SOL * LAMPORTS_PER_SOL) as u128;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 3577228796);
        // --------------------------------------- the max
        //10 bn sol
        let lamp = (10 * LAMPORTS_PER_SOL * LAMPORTS_PER_SOL) as u128;
        let keys = sol_to_keys(lamp).unwrap();
        assert_eq!(keys, 11313228509);
        // --------------------------------------- upper bound
        let lamp = (10 * LAMPORTS_PER_SOL * LAMPORTS_PER_SOL + 1) as u128;
        let keys = sol_to_keys(lamp);
        assert!(keys.is_err());
    }
}
