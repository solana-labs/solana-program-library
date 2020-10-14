---
title: Shared memory Program
---

A simple program and highly optimized program that writes instruction data into
the provided account's data

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Shared memory Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

## Interface

The Shared memory program expects one account and writes instruction data into
the account's data.  The first 8 bytes of the instruction data contain the
little-endian offset into the account data.  The rest of the instruction data is
written into the account data starting at that offset.  

## Operational overview

This program is useful for returning data from cross-program invoked programs to
the invoker.  Because the account does not need to be signed it is not reliable
to use this program to pass data between programs from different transactions.
