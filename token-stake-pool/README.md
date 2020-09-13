# token-stake-pool program

A program for pooling together SOL to be staked by an of-chain agent
running stake-o-matic .

1. Network rewards are increased when the stake is delegated to low
staked nodes.

2. Agent charges a fee as a percentage of rewards that are above the
median rewaard rate.

Once slashing is enabled, these two factors should be the driver for
decentralization.

1. Agent wants to increase the reward rate, and therefore delegate the
pool to the largest number of nodes.

2. Agent wants to avoid slashing to maintain a reward rate above the
median.

Full documentation is available at https://spl.solana.com

Javascript binding are available in the `./js` directory.
