# Binary Option

This protocol is a primitive version of a binary options. Participants can enter a long or short position depending on their conviction. (These sides are set completely arbitrarily). The eventual goal is to have a higher level program manage the pool and handle settlements with an oracle based voting approach. Every bet in the pool involves 2 parties (1 long and 1 short) each depositing some collateral. Because bets are made on binary event, the sum of collateral will be equal to a multiple of some power of 10 (`10 ** N` where `N` is configurable).

The module contains the Rust implementation of protocol as well as a Python client and test suite.

Suppose we had a binary option on the winner of the 2021 NBA Finals (Phoenix Suns vs. Milwaulkee Bucks). At the time of writing this (July 9th, 2021), the moneyline spread is -190 Suns +170 Bucks. This backs out an implied probability of approximately 36% that the Bucks win the championship. Suppose our binary option was on the Bucks winning this series, and that it is denominated by some wrapped stablecoin WUSD (dollar pegged) where every contract settled to 10000 WUSD (`N = 4` corresponding to 1 cent granularity). You observe that someone is willing to go short Bucks for 10 contracts at 3000 WUSD (less than the estimated probability of 36%). You can take on the opposite trade by buying 10 long contracts on the Bucks for 3000.

This invokes a `Trade` instruction with size 10, buy_price 3000, and sell_price 7000. Note that these prices must sum to 10000. As part of the protocol, you transfer 30000 WUSD into the binary option and the counterparty deposits 70000 (assuming that both parties start with 0 position). In return, 10 long tokens (minted by the contract) are added to your account, and 10 short tokens are minted to your counterparty's account.

Now suppose the Bucks win Game 3, and the estimated probability of the Bucks winning the series jumps to 40%. You offer to sell all of your contracts for 4000 WUSD, and you get a buyer. Because you already hold long tokens, the contract will burn those existing tokens, and you are transferred 40000 WUSD from the pool. If your counterparty is currently short 1 contract, they pull out 6000 WUSD from the pool (exiting out of their short position) and deposit 9 * 4000 = 36000 WUSD (buying into their long position). In total, the pool collateral changes by -40000 + 36000 - 6000 = -10000 WUSD or exactly 1 contract! After the dust settles, you walk away with no position and a net profit of 10000 WUSD ($100).

We'll discuss this mechanism in more detail later.

## Client Setup 
First, clone down the repository (TODO publish to PyPI)

Create a virtual environment and and install the dependencies in `client/requirements.txt`

```
python3 -m virtualenv venv
source venv/bin/activate
pip install -r client/requirements.txt
```

To run the tests against the program code deployed on devnet, run:
```
python -m client.test
```

# Instructions

### InitializeBinaryOption
`InitializeBinaryOption` creates a new binary option where the denominated decimals are specified as arguments. (The "escrow" mint is included in the list of accounts). New mints are created for long and short tokens, and the ownership of these mints is transferred to a program derived address.

### Trade
`Trade` handles all of the complicated wiring of a wager being added to the pool. This is tricky because the existing positions of the participants needs to be accounted for. There are 3 variables we care about: 

`n` the number of contracts traded

`n_b` the number of short contracts owned by the buyer

`n_s` the number of long contracts owned by the seller

We know from our college combanatorics/discrete math class that there are 3! = 6 ways to order 3 items. Let's list out all configurations of how these numbers can bet ordered from largest to smallest (assuming all distinct):

```
1) n_b > n_s > n
2) n_s > n_b > n
3) n   > n_b > n_s
4) n   > n_s > n_b
5) n_b > n   > n_s
6) n_s > n   > n_b
```
This is a lot of cases to consider, but we can group them into combined categories:
```
n_b >= n && n_s >= n
```
This clause essentially groups 1) and 2) together. In this case, both buyer and seller are simply reducing their existing inventory. Therefore, we can just remove `n` long tokens and `n` short tokens from circulation. Both parties are also entitled to the locked up funds for their positions that were closed, so the buyer receives `n * sell_price` and the seller received `n * buy_price`. This might be confusing at first, but a good way to think about this is that there are no "sellers". Everyone with inventory is a "buyer". If an event as a probability `p` of occurring, the buyer is paying `p` and the seller is paying `1-p`. When a market participant receives funds, they are "selling out" (either locking in profits or losses) of their existing position.

```
n_b < n && n_s < n
```
This clause groups 2) and 3) together (most complex). In this case, both buyer and seller swap positions -- the buyer goes from short to long and the seller goes from long to short. We will first burn the tokens all exiting tokens for parties and then mint new tokens to ensure the buyer's change is `+n` and the seller's change is `-n`. Both parties are also entitled to the locked up funds for their positions that were closed (`n_b * sell_price` for the buyer and `n_s * buy_price` for the seller). The net change in tokens can be calculated as follows: `(-n_b - n_s + 2n - n_b - n_s) / 2 = n - n_b - n_s`. If this quantity is positive, this means that the trade causes a net increase in the total supply of contracts in the binary option. Otherwise, it results in a net decrease in total circulation.

```
n_b >= n && n_s <  n
```
This is equivalent to clause 5. The buyer has swapped positions, and the seller has reduced inventory. Like before, we will burn and mint tokens such the buyer's net change in position is `+n` and the seller's net change is `-n`. Both parties are also entitled to the locked up funds for their positions that were closed. The net change in tokens can be calculated as follows: `(-n - n_s + n - n_s) / 2 = -n_s`. This always results in a decrease in total circulation.

```
n_b <  n && n_s >= n
```
It's easy to see that this is almost identical to the previous case. The net circulation decreases by `n_b`. This proof is left as an exercise to the reader.

When all of the dust settles, the pool participants can enter and exit their positions while the pool is live, and the pool will always be fully collateralized!

### Settle
`Settle` is invoked when a winner of the bet is decided. This, in theory, should be done through an oracle by the higher level protocol that uses this primitive (composability effects). Once an event is settled, no more trades can occur. One TODO is to potentially add another stage -- first stop trading and settle as a gradual process

### Collect
`Collect` is invoked when retrieving funds from a pool after it has fully settled. All of the user's tokens are burned and if they have any of the winning token, the user will receive a proportional stake of the pool (`(# tokens / total circulation) * size of pool`). The circulation of the pool is then reduced to reflect a global change in stake of all participants who have yet to retrieve their funds.
