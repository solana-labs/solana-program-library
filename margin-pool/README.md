Margin trading for token-swap

1. Takes deposits of LP tokens that are used to open positions.
2. On demand converts LP tokens to the requested underlying and opens a position.
3. Margin collateral has to be greater than the price movement in the market for the margin trade
4. Margin collateral has to cover the loss of fees

Collateral for the margin trade needs to cover how much the price moves when the trade occurs.

1. user borrows a bunch of USDC from aave/lending pool
2. BUYs $100 worth of BTC, moves the price on btc/usdc swap (user now has 100 btc from swap, needs to pay back aave usdc)
3. deposits $X collateral into the margin pool, and does an atomic BUY of $100 btc on the btc/usdc swap
4. Internally some LP tokens are converted to $100 USDC
5. $100 worth of BTC is bought with the swap
6. sells the AAVE btc in the swap at a higher price due to buy with #3 (now has USDC)
7. pays back AAVE to close the usdc loan
8. user gives up the $X collateral used in #3 since price has dropped

The deposit $X used in #3 needs to be higher than the price difference for the trade that closes the aave loan.  
The margin pool takes LP tokens as a deposit, and the “funding rate” needs to cover the loss of fees while the position is open.
Opening positions causes some liquidity to be removed from the swap, which would increase the margin requirements.

