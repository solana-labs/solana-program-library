---
title: Project Status
---

The Token-2022 program is still under audit and not meant for full production use.
In the meantime, all clusters have the latest program deployed **for testing and
development purposes ONLY**.

## Timeline

Here is the general program timeline and rough ETAs:

| Issue                       | ETA                          |
| --------------------------- | ---------------------------- |
| Code-complete & final audit | Summer 2023                  |
| Mainnet recommendation      | Fall 2023 (depends on v1.16) |
| Freeze program              | 2024                         |

More information: https://github.com/orgs/solana-labs/projects/34

## Remaining items

### v1.16 with curve syscalls

In order to use confidential tokens, the cluster must run at least version 1.16
with the elliptic curve operations syscalls enabled.

More information: https://github.com/solana-labs/solana/issues/29612

### Zero-knowledge proof split

In order to use confidential tokens, the cluster must run at least version 1.16
with the ZK Token proof program enabled.

More information: https://github.com/solana-labs/solana/pull/32613

The ZK Token proof program was recently updated to support the splitting of
the longer zero-knowledge proofs into smaller components. The
token-2022 program is in the process of active development to properly process
the new proof format.

More information: https://github.com/solana-labs/solana-program-library/issues/4817

## Future work

### Wallets

To start, wallets need to properly handle the token-2022 program and its accounts,
by fetching token-2022 accounts and sending instructions to the proper program.

Next, to use confidential tokens, wallets need to create zero-knowledge proofs,
which entails a new transaction flow.

### Increased transaction size

To support confidential transfers in one transaction, rather than split up over
multiple transactions, the Solana network must accept transactions with a larger
payload.

More information: https://github.com/orgs/solana-labs/projects/16

## Upgradability

To facilitate deploying updates and security fixes, the program deployment remains
upgradable. Once audits are complete and the program has been stable for six months,
the deployment will be marked final and no further upgrades will be possible.
