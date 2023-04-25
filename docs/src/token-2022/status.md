---
title: Project Status
---

The Token-2022 program is still under audit and not meant for full production use.
In the meantime, all clusters have the latest program deployed **for testing and
development purposes ONLY**.

## Timeline

Here is the general program timeline and rough ETAs:

| Issue | ETA |
| --- | --- |
| Finish program | end of May |
| Final audit | end of June |
| Mainnet recommendation | middle of July |
| Freeze program | Q1 2024 |

More information: https://github.com/orgs/solana-labs/projects/34

## Remaining items

### v1.14 with curve syscalls

In order to use confidential tokens, the cluster must run at least version 1.14
with the elliptic curve operations syscalls enabled.

More information: https://github.com/solana-labs/solana/issues/29612

### Zero-knowledge proof split

To fit within the current transaction size limits, the zero knowledge proofs must
be split into their component parts and uploaded to the chain through multiple
transactions. Once the proofs exist, the user can issue a transfer and clean up
the proofs.

After splitting the proofs in the zero-knowledge token SDK, the token-2022 program
must properly consume the new proof format.

More information: https://github.com/solana-labs/solana/pull/30816

### Permissioned-transfer extension

There are many mutually exclusive solutions for restricting transfer of tokens,
that all require different sets of accounts to validate a transfer.

The permissioned-transfer extension allows a mint creator to specify a program
that must be called to validate transfers. The program must conform to the
permissioned-transfer program interface.

More information: https://github.com/solana-labs/solana-program-library/pull/4105

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
