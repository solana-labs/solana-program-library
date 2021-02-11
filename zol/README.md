# ZOL Program for confidential payments

The ZOL program is a work-in-progress, but aims to be an implementation of
Zether confidential payments for the Solana blockchain.

Currently, it doesn't do solvency or equivalence proof verification and so
is no way useable today. Furthermore, users don't yet inject randomness, so
it is straightforward to decipher the encrypted amounts by simply encrypting
each SOL amounts starting with zero until the result matches the on-chain
encrypted amount.
