---
title: Stake Pool Introduction
---

A program for pooling together SOL to be staked by an off-chain agent running
a Delegation Bot which redistributes the stakes across the network and tries
to maximize censorship resistance and rewards.

| Information | Account Address |
| --- | --- |
| Stake Pool Program | `SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy` |

NOTE: The devnet deployment of the program still uses v0.6.4, and is not suitable
for testing. For testing, it is recommended to use testnet, a local test validator,
or deploy your own version for devnet.

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
available at [docs.rs](https://docs.rs/spl-stake-pool/latest/spl_stake_pool/index.html).

## Security Audits

Multiple security firms have audited the stake pool program to ensure total
safety of funds. The audit reports are available for reading, presented in descending
chronological order, and the commit hash that each was reviewed at:

* Quantstamp
    - Initial review commit hash [`99914c9`](https://github.com/solana-labs/solana-program-library/tree/99914c9fc7246b22ef04416586ab1722c89576de)
    - Re-review commit hash [`3b48fa0`](https://github.com/solana-labs/solana-program-library/tree/3b48fa09d38d1b66ffb4fef186b606f1bc4fdb31)
    - Final report https://github.com/solana-labs/security-audits/blob/master/spl/QuantstampStakePoolAudit-2021-10-22.pdf
* Neodyme
    - Review commit hash [`0a85a9a`](https://github.com/solana-labs/solana-program-library/tree/0a85a9a533795b6338ea144e433893c6c0056210)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/NeodymeStakePoolAudit-2021-10-16.pdf
* Kudelski
    - Review commit hash [`3dd6767`](https://github.com/solana-labs/solana-program-library/tree/3dd67672974f92d3b648bb50ee74f4747a5f8973)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/KudelskiStakePoolAudit-2021-07-07.pdf
* Neodyme Second Audit
    - Review commit hash [`fd92ccf`](https://github.com/solana-labs/solana-program-library/tree/fd92ccf9e9308508b719d6e5f36474f57023b0b2)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/NeodymeStakePoolAudit-2022-12-10.pdf
* OtterSec
    - Review commit hash [`eba709b`](https://github.com/solana-labs/solana-program-library/tree/eba709b9317f8c7b8b197045161cb744241f0bff)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/OtterSecStakePoolAudit-2023-01-20.pdf
* Neodyme Third Audit
    - Review commit hash [`b341022`](https://github.com/solana-labs/solana-program-library/tree/b34102211f2a5ea6b83f3ee22f045fb115d87813)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/NeodymeStakePoolAudit-2023-01-31.pdf
* Halborn
    - Review commit hash [`eba709b`](https://github.com/solana-labs/solana-program-library/tree/eba709b9317f8c7b8b197045161cb744241f0bff)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/HalbornStakePoolAudit-2023-01-25.pdf
* Neodyme Fourth Audit
    - Review commit hash [`6ed7254`](https://github.com/solana-labs/solana-program-library/tree/6ed7254d1a578ffbc2b091d28cb92b25e7cc511d)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/NeodymeStakePoolAudit-2023-11-14.pdf
* Halborn Second Audit
    - Review commit hash [`a17fffe`](https://github.com/solana-labs/solana-program-library/tree/a17fffe70d6cc13742abfbc4a4a375b087580bc1)
    - Report https://github.com/solana-labs/security-audits/blob/master/spl/HalbornStakePoolAudit-2023-12-31.pdf
