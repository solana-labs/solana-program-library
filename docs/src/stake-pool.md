---
title: Stake Pool Introduction
---

A program for pooling together SOL to be staked by an off-chain agent running
a Delegation Bot which redistributes the stakes across the network and tries
to maximize censorship resistance and rewards.

| Information | Account Address |
| --- | --- |
| Stake Pool Program | `SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy` |

## Getting Started

To get started with stake pools:

- [Install the Solana Tools](https://docs.solana.com/cli/install-solana-cli-tools)
- [Install the Stake Pool CLI](stake-pool/cli.md)
- [Step through the quick start guide](stake-pool/quickstart.md)
- [Learn more about stake pools](stake-pool/overview.md)
- [Learn more about fees and monetization](stake-pool/fees.md)

## Source

The Stake Pool Program's source is available on
[GitHub](https://github.com/solana-labs/solana-program-library/tree/master/stake-pool).

For information about the types and instructions, the Stake Pool Rust docs are
available at [docs.rs](https://docs.rs/spl-stake-pool/0.6.3/spl_stake_pool/).

## Security Audits

Multiple security firms have audited the stake pool program to ensure total
safety of funds. The audit reports are available for reading, presented in descending
chronological order, and the commit hash that each was reviewed at:

* Quantstamp
    - Initial review commit hash [`99914c9`](https://github.com/solana-labs/solana-program-library/tree/99914c9fc7246b22ef04416586ab1722c89576de)
    - Re-review commit hash [`3b48fa0`](https://github.com/solana-labs/solana-program-library/tree/3b48fa09d38d1b66ffb4fef186b606f1bc4fdb31)
    - Final report https://solana.com/SolanaQuantstampStakePoolAudit.pdf
* Neodyme
    - Review commit hash [`0a85a9a`](https://github.com/solana-labs/solana-program-library/tree/0a85a9a533795b6338ea144e433893c6c0056210)
    - Report https://solana.com/SolanaNeodymeStakePoolAudit.pdf
* Kudelski
    - Review commit hash [`3dd6767`](https://github.com/solana-labs/solana-program-library/tree/3dd67672974f92d3b648bb50ee74f4747a5f8973)
    - Report https://solana.com/SolanaKudelskiStakePoolAudit.pdf
