# stake-pool program

A work-in-progress program for pooling together SOL to be staked by an off-chain
agent running SoM (Stake-o-Matic).

Each SoM needs at least one pool.  Users deposit stakes into the SoM pool
and receives a pool token minus the fee.  The SoM redistributes the stakes
across the network and tries to maximize censorship resistance and rewards.

Full documentation is available at https://spl.solana.com/stake-pool

Javascript bindings are available in the `./js` directory.
