# token-stake-pool program

A program for pooling together SOL to be staked by an of-chain agent
running SoM (Stake-o-Matic).

Each SoM needs at least one pool.  User's deposit stakes into the SoM
pool and receive a pool token.  The SoM redistributes the stakes across
the network and tries to maximize censorship reistance and rewards.

Full documentation is available at https://spl.solana.com

Javascript binding are available in the `./js` directory.
