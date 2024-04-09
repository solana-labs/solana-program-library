---
title: Quick Start Guide
---

The Token-2022 program provides confidential transfer functionality through the
confidential transfer extension.

This guide explains how to use the confidential transfer extension.

Please see the [Token-2022 Introduction](../token-2022) for more general information
about Token-2022 and the concept of extensions.

## Setup

See the [Token Setup Guide](../token#setup) to install the client utilities.
Token-2022 shares the same CLI and NPM packages for maximal compatibility.

All of the commands here exist in a
[helper script](https://github.com/solana-labs/solana-program-library/tree/master/token/cli/examples/confidential-transfer.sh)
at the
[Token CLI Examples](https://github.com/solana-labs/solana-program-library/tree/master/token/cli/examples).

### Example: Create a mint with confidential transfers

To create a new mint with confidential transfers enabled, run:

```console
$ spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb create-token --enable-confidential-transfers auto
```

The `auto` keyword means that any token user can permissionlessly configure their
account to perform confidential transfers.

If you would like to gate confidential transfer functionality to certain users,
you can set the approve policy to `manual`. With this approve policy, all users
must be manually approved to perform confidential transfers. Anyone can still use
the token non-confidentially.

Note that you must configure your mint with confidential transfers at creation,
and cannot add it later.

### Example: Configure a token account for confidential transfers

Account creation works as normal:

```console
$ spl-token create-account <MINT_PUBKEY>
```

Once the user creates their account, they may configure it for confidential transfers:

```console
$ spl-token configure-confidential-transfer-account --address <ACCOUNT_PUBKEY>
```

Note that only the account owner may configure confidential transfers for their
account: only they should set the encryption key for their account. This is
different from normal accounts, such as associated-token-accounts, where someone
can create another person's account.

### Example: Deposit confidential tokens

Once the user configures their account for confidential transfers and has a
non-confidential token balance, they must deposit their tokens from non-confidential
to confidential:

```console
$ spl-token deposit-confidential-tokens <MINT_PUBKEY> <AMOUNT> --address <ACCOUNT_PUBKEY>
```

Note that the deposited tokens will no longer exist on the account's non-confidential
balance: they have been completely moved into the confidential balance.

### Example: Apply pending balance

Whenever an account receives confidential tokens from transfers or deposits, the
balance will appear in the "pending" balance, which means that the user cannot
immediately access the funds.

To move a balance from "pending" to "available", simply run:

```console
$ spl-token apply-pending-balance --address <ACCOUNT_PUBKEY>
```

### Example: Transfer confidential tokens

Once an account has an available balance, a user may finally transfer the tokens
to another account that has been configured for confidential transfers!

```console
$ spl-token transfer <MINT_PUBKEY> <AMOUNT> <DESTINATION_PUBKEY> --confidential
```

This operation takes a little bit longer since it requires multiple dependent
transactions, but it's still only a few seconds.

### Example: Withdraw confidential tokens

A user whose account has an available confidential balance may withdraw those
tokens back into their non-confidential balance.

```console
$ spl-token withdraw-confidential-tokens <MINT_PUBKEY> <AMOUNT> --address <ACCOUNT_PUBKEY>
```

Be sure to apply any pending balance before running this command to be sure that
all tokens are available.
