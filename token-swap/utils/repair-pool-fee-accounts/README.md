# Overview

A new instruction was added to token-swap to reset the fee account on a token swap if it was incorrect.  This utility calls that instruction for all "broken" token swaps.

# Usage

`cargo run` 

If busted token-swap fee payer accounts are found, you'll be prompted to type "repair".  It is suggested that you verify the token account is indeed deleted for that swap first, even though the onchain program won't allow you to fix a non-busted account. 

# Configuration

This program uses your solana cli configured rpx node for rpc calls, and your keypair as the payer.  No special config outside `solana config ...` is required.

`cargo run --help` shows some overridable options, but for mainnet Step Finance token swaps, that shouldn't be needed.