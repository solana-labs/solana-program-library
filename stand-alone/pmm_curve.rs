
/**
 * Proactive market making curve implementation (interview question for solana-labs)
 * (c) Derek Anderson <anderson.derek@gmail.com>
 *
 * i: the market price provided by an oracle
 * k: [0, 1], where 0 is a constant price and 1 is the AMM curve
 * b_0: base token regression target - total number of base tokens deposited by liquidity providers
 * b: base token balance - number of base tokens currently in the pool
 * q_0: quote token regression target - total number of quote tokens deposited by liquidity providers
 * q: quote token balance - number of quote tokens currently in the pool
 *
 * see https://github.com/solana-labs/solana-program-library/blob/master/token-swap/proposals/ProactiveMarketMaking.md
 * and https://dodoex.github.io/docs/docs/pmmDetails/
 */

fn p_margin(i: f64, b: u128, b_0: u128, q: u128, q_0: u128, k: f64) -> f64 {
  let mut r = 1.0;
  if b < b_0 {
    r = 1.0 - k + (b_0 as f64/b as f64).powf(2.0) * k;
  } else
  if q < q_0 {
    r = 1.0 / (1.0 - k + (q_0 as f64/q as f64).powf(2.0) * k)
  }
  return i*r;
}

fn main() {
  println!("please run: rustc pmm_curve.rs --test && ./pmm_curve");
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_constant_price() {
    // if k=0, price should always match the oracle's price
    assert_eq!(p_margin(15.0, 1, 1, 1, 1, 0.0), 15.0);
  }
  
  #[test]
  fn test_constant_price_small_b() {
    // b < b_0
    assert_eq!(p_margin(15.0, 1, 100, 1, 1, 0.0), 15.0);
  }
  
  #[test]
  fn test_constant_price_small_q() {
    // q < q_0
    assert_eq!(p_margin(15.0, 1, 1, 1, 100, 0.0), 15.0);
  }
  
  #[test]
  fn test_constant_price_zeros() {
    // should not divide by zero
    assert_eq!(p_margin(15.0, 0, 0, 0, 0, 0.0), 15.0);
  }
  
  #[test]
  fn test_pmm_equiv_to_amm_base() {
    // k=1, so price should scale linearly according to AMM
    assert_eq!(p_margin(15.0, 1, 1, 1, 1, 1.0), 15.0);
  }
  
  #[test]
  fn test_pmm_equiv_to_amm_small_b() {
    // b < b_0
    assert_eq!(p_margin(15.0, 1, 2, 0, 0, 1.0), 60.0);
  }
  
  #[test]
  fn test_pmm_equiv_to_amm_small_q() {
    // q < q_0
    assert_eq!(p_margin(16.0, 0, 0, 1, 2, 1.0), 4.0);
  }
  
  #[test]
  fn test_nonlinear_small_b() {
      assert_eq!(p_margin(10.0, 2, 4, 0, 0, 0.5), 25.0);
  }
  
  #[test]
  fn test_nonlinear_small_q() {
      assert_eq!(p_margin(10.0, 0, 0, 2, 4, 0.5), 4.0);
  }
  
}

