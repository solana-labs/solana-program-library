---
title: Shared memory Program
---

A simple program and highly optimized program that writes the instruction data
into the provided account's data

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Shared memory Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

## Interface

The shared memory program expects a single account, owned by the shared memory program.  The account's data
must be large enough to hold the entire instruction data.

## Operational overview

The Shared memory program directly writes all the instruction data into the
provided account's data.  It is useful for returning data from cross-program
invoked programs to the invoker.  Because the account does not need to be signed
it is not reliable to use this program to pass data between programs from
different transactions.
