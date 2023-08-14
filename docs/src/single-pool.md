---
title: Single-Validator Stake Pool
---

Trustless liquid staking for all Solana validators.

## Overview

The single-validator stake pool program is an upcoming SPL program that enables liquid staking with zero fees, no counterparty, and 100% capital efficiency.

The program defines a canonical pool for every vote account, which can be initialized permissionlessly, and mints tokens in exchange for stake delegated to its designated validator.

The program is a stripped-down adaptation of the existing multi-validator stake pool program, with approximately 80% less code, to minimize execution risk.

## Source

The Single Pool Program's source is available on
[GitHub](https://github.com/solana-labs/solana-program-library/tree/master/single-pool/program).

## Security Audits

The Single Pool Program has received two audits to ensure total safety of funds:

* Neodyme (2023-08-08)
    - Review commit hash [`735d729`](https://github.com/solana-labs/solana-program-library/commit/735d7292e35d35101750a4452d2647bdbf848e8b)
    - Final report https://github.com/solana-labs/security-audits/blob/master/spl/NeodymeSinglePoolAudit-2023-08-08.pdf
* Zellic (2023-06-21)
    - Review commit hash [`9dbdc3b`](https://github.com/solana-labs/solana-program-library/commit/9dbdc3bdae31dda1dcb35346aab2d879deecf194)
    - Final report https://github.com/solana-labs/security-audits/blob/master/spl/ZellicSinglePoolAudit-2023-06-21.pdf
