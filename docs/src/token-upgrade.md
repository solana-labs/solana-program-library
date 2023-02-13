---
title: Token Upgrade Program
---

The Token Upgrade Program provides a stateless protocol for permanently converting
tokens from one mint to another.

The program provides a simple mechanism for burning the original tokens and receiving
an equal number of new tokens from an escrow account controlled by the program.

## Audit

The repository [README](https://github.com/solana-labs/solana-program-library#audits)
contains information about program audits.

## Background

Token-2022 contains many new features for mint owners to customize the behavior
of their tokens. You can find full information about Token-2022 and its extensions
in the [documentation](token-2022.md).

Mint owners may want to take advantage of new functionality for their users, but
there is no way to automatically convert tokens from Token to Token-2022.

The Token Upgrade Program defines an escrow authority, a program-derived address
from two addresses, the original and new mints. Any new token account owned by or
delegated to this escrow authority may be used as the escrow account.

A holder of original tokens provides their original token account, a new token account,
and the escrow account. If the escrow account has enough tokens, the protocol will
burn the original tokens and transfer the same amount of new tokens to the user's
new account.

The program ensures that the decimals of both mints are the same, so if the mints
have different decimals, the upgrade fails.

The program is completely stateless and has a simple implementation, so mint owners
may customize it with additional functionality. For example, if they want to
upgrade between mints with different decimals, they can define how to scale
the transferred number up or down as desired.

**Note**: The Token Upgrade Program can also exchange tokens that belong to the
same program, but different mints. For example, a mint owner can provide an upgrade
between two Token-2022 mints. This is useful if the mint owner wants to add new
functionality to their mint.

## Source

The Token Upgrade Program's source is available on
[GitHub](https://github.com/solana-labs/solana-program-library)

## Interface

The Token Upgrade Program is written in Rust and available on
[crates.io](https://crates.io/crates/spl-token-upgrade) and
[docs.rs](https://docs.rs/spl-token-upgrade).

## Command-line Utility

The `spl-token-upgrade` command-line utility can be used to manage token upgrades.
Once you have [Rust installed](https://rustup.rs/), run:

```sh
$ cargo install spl-token-upgrade-cli
```

Run `spl-token-upgrade --help` for a full description of available commands.

### Configuration

The `spl-token-upgrade` configuration is shared with the `solana` command-line tool.

## Token Upgrade Process

This section describes how to upgrade tokens from Token to Token-2022, for the
mint owner and token holders.

This guide also uses the `spl-token` command-line tool. Please see the full
[Token documentation](token.mdx) for more info.

### Setup

A mint owner has a mint associated with the Token program at address
`o1d5Jt8z8vszx4FJ2gNJ3FZH34cer9sbparg7GVt7qm`, and they want `o1d` token holders
to upgrade to `NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ` tokens, associated with
Token-2022.

### Create token escrow

The command-line tool allows anyone to create a new token account owned by the
escrow authority, given the original and new mint addresses:

```sh
$ spl-token-upgrade create-escrow o1d5Jt8z8vszx4FJ2gNJ3FZH34cer9sbparg7GVt7qm NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ
Creating escrow account 2mW9oGUbaJiCHtkhN5TNTaucY2ziJmAdcJtp5Ud6m4Jy owned by escrow authority A38VXB1Qgssz2qkKgzEkyZNQ27oTuy18T6tA9HRP5mpE
Signature: 4tuJffE4DTrsXb7AM3UWNjd286vyAQcvhQaSKPVThaZMzaBiptKCKudaMWjbbygTUEaho87Ar288Mih5Hx6PpKke
```

**Note**: The command-line tool creates the associated token account for the escrow
authority, but any token account for `NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ`
is usable for upgrades, as long as the account is owned by or delegated to the
escrow authority.

### Add tokens to the escrow account

With the escrow account created, the mint owner must now add tokens to that account.
They can do this by minting new tokens or transferring existing tokens.

```sh
$ spl-token mint NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ 1000 2mW9oGUbaJiCHtkhN5TNTaucY2ziJmAdcJtp5Ud6m4Jy
```

Or:

```sh
$ spl-token transfer NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ 1000 2mW9oGUbaJiCHtkhN5TNTaucY2ziJmAdcJtp5Ud6m4Jy
```

### Upgrade original tokens into new tokens

With all accounts in place, any original token holder may redeem new tokens
whenever they want.

First, they must create a new token account to receive the tokens:

```sh
$ spl-token create-account NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ
```

Next, they perform the exchange:

```sh
$ spl-token-upgrade exchange o1d5Jt8z8vszx4FJ2gNJ3FZH34cer9sbparg7GVt7qm NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ
Burning tokens from account 4YfpfMzHYCCYVBJqvTG9VtTPLMuPzVBi77aMRxVB4TDg, receiving tokens into account JCaWYSvLZkja51RbToWBaV4kp1PhfddX64cTLUqpdMzE
Signature: 3Zs1PtMV7XyRpfX9k7cPg7Hd43URvBD3aYEnd6hb5deKvSWXrEW5yoRaCuqtYJSsoa2WtkdprTsHEh3VLYWEGhkb
```

The tool defaults to using associated token accounts for the user on the original
and new token mints, and for the escrow authority on the new mint. It's possible
to specify each of these individually:

```sh
$ spl-token-upgrade exchange o1d5Jt8z8vszx4FJ2gNJ3FZH34cer9sbparg7GVt7qm NewnQeoDG4BbHRCodgjscuypfXdiixcWDPyLiseziQZ --burn-from 4YfpfMzHYCCYVBJqvTG9VtTPLMuPzVBi77aMRxVB4TDg --destination JCaWYSvLZkja51RbToWBaV4kp1PhfddX64cTLUqpdMzE --escrow 2mW9oGUbaJiCHtkhN5TNTaucY2ziJmAdcJtp5Ud6m4Jy
Burning tokens from account 4YfpfMzHYCCYVBJqvTG9VtTPLMuPzVBi77aMRxVB4TDg, receiving tokens into account JCaWYSvLZkja51RbToWBaV4kp1PhfddX64cTLUqpdMzE
Signature: 3P4o4Fxnm4yvB9i6jQzyniqNUqnNLsaQZmCw5q5n5J8nwv9wxJ73ZRYH3XNFT4ferDbCXMqc5egCkhZEkyfCxhgC
```

After the upgrade, the user may clean up the old token account to recover the
rent-exempt lamports.

```sh
$ spl-token close o1d5Jt8z8vszx4FJ2gNJ3FZH34cer9sbparg7GVt7qm
```
