---
title: Project Status
---

All clusters have the latest program deployed **without confidential transfer
functionality**.

The program with confidential transfer functionality will be deployed once
Agave v2.0 reaches mainnet-beta with the appropriate cluster features enabled.

## Timeline

Here is the general program timeline and rough ETAs:

| Issue                       | ETA                            |
| --------------------------- | ------------------------------ |
| Mainnet recommendation      | Winter 2024 (depends on v1.17) |
| Token group extension       | Summer 2024                    |
| Confidential transfers      | Autumn 2024 (depends on v2.0)  |
| Freeze program              | 2025                           |

More information: https://github.com/orgs/solana-labs/projects/34

## Remaining items

### v2.0 with ZK ElGamal Proof Program

In order to use confidential tokens, the cluster must run at least version 2.0
with the ZK ElGamal Proof Program enabled.

More information: https://github.com/anza-xyz/agave/issues/1966

## Future work

### Wallets

To start, wallets need to properly handle the Token-2022 program and its accounts,
by fetching Token-2022 accounts and sending instructions to the proper program.

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
