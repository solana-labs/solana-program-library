---
title: Program updates
---

As programs are enhanced and patched the accounts associated with the older
versions must be migrated forward.

Solana programs are stored in immutable accounts to ensure consistent behavior
for all time.  Solana also prevents accounts populated with data to be assigned
to other programs to ensure only the owning program has modified the data
and eliminates the ability for malicious programs to falsify data.  One side effect
of these two restrictions is that updating a program becomes more difficult
then simply replacing it on-chain or moving accounts owned by the old versions to
the newest version.

## Updating programs by proxy

One method of upgrading programs is to do so by proxy via a migration program.
The migration program can perform checks and issue instructions to the old and new
versions of the programs in order to ferry data from accounts owned by the old
version to accounts owned by the new.  In order to support the proxy model, both
the new and old programs must support some kind of end-of-life mechanism and
some kind of beginning-of-life mechanism for accounts they own.

A typical migration program would do the following

- Be passed:
  - The old account to migrate and any required authorities/signers
  - The new account and any required authorities/signers /signers
  - Any migration specific data
- Issues instructions to the old program to deinitialize the old account
- Issue instructions to the new program to create and initialize the new account

The result of the migration program should be a deinitilized account that is no
longer usable by the old program and a new account owned by the new program and
is informationally equivalent to the old account but in a form that is usable by
the new program.

The exact mechanism that any particular program should use to migrate from one
form to another is up to the program and will depend on the nature of the
program and its data.

For an example of how the SPL Token program migrates accounts from spl-token to
spl-token-v3 refer to [SPL Token program
update](token.md#program-updates)



