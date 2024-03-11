---
title: Project Status
---

All clusters have the latest program deployed **without confidential transfer
functionality**.

The program with confidential transfer functionality will be deployed once
Solana v1.17 reaches mainnet-beta with the appropriate syscalls enabled.

## Timeline

Here is the general program timeline and rough ETAs:

| Issue                       | ETA                            |
| --------------------------- | ------------------------------ |
| Mainnet recommendation      | Winter 2024 (depends on v1.17) |
| More ZK features            | Spring 2024 (depends on v1.18) |
| Freeze program              | 2024                           |

More information: https://github.com/orgs/solana-labs/projects/34

## Remaining items

### v1.17 with curve syscalls

In order to use confidential tokens, the cluster must run at least version 1.17
with the elliptic curve operations syscalls enabled.

More information: https://github.com/solana-labs/solana/issues/29612

### Zero-knowledge proof split

In order to use confidential tokens, the cluster must run at least version 1.17
with the ZK Token proof program enabled.

More information: https://github.com/solana-labs/solana/pull/32613

## Future work

### Confidential transfers with fee

Due to the transaction size limit, it is not possible to do confidential transfers
with a fee. We plan to include that capability with Solana 1.18.

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
