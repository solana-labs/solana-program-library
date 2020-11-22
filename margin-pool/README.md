Margin trading for token-swap

1. Takes deposits of LP tokens that are used to open positions.
2. On demand converts LP tokens to the requested underlying and opens a position.
3. Margin collateral has to be greater than the price movement in the market for the margin trade
4. Margin collateral has to cover the loss of fees

Collateral for the margin trade needs to cover how much the price moves when the trade occurs.

1. user borrows a bunch of USDC from lending pool
2. BUYs $100 worth of BTC, moves the price on btc/usdc swap (user now has 100 btc from swap, needs to pay back lending pool usdc)
3. deposits $X collateral into the margin pool, and does an atomic BUY of $100 btc on the btc/usdc swap
4. Internally some LP tokens are converted to $100 USDC
5. $100 worth of BTC is bought with the swap
6. sells the AAVE btc in the swap at a higher price due to buy with #3 (now has USDC)
7. pays back AAVE to close the usdc loan
8. user gives up the $X collateral used in #3 since price has dropped

The deposit $X used in #3 needs to be higher than the price difference for the trade that closes the lending pool loan.  
The margin pool takes LP tokens as a deposit, and the “funding rate” needs to cover the loss of fees while the position is open.
Opening positions causes some liquidity to be removed from the swap, which would increase the margin requirements.


For example:

* Pool has 50btc at 20k, invariant = 50 * (50 * 20,000) = 50,000,000 
* Cost to go long `10btc = 50,000,000/40 - (1000000 + 10*20,000) = 50,000 = 4x leverage`
* Cost to go long `1btc = 50,000,000/49 - (1000000 + 1*20,000) = 408.1632653061 = 49x leverage`

Position:

* Collateral, intial deposit to open the position.
* Borrow amount, the size of the position.
* Position token, represents the position opened by the user.
* Borrow token, this token covers the risk and generates yield.  Yield is split with the pool based on the ratio of collateral / borrow.

Position can be reduced by burning the Position token and repaying the debt.

Liqudiations:

Positions can be liquidated in order of least collaterlized to most collaterlized.

Debt Pools:

Since liquidations are ordered, the A or B debt can be sold as aggregated debt. Yield is split between the LP pool and debt pools by the collateral atio.
