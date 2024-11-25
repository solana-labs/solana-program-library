NOTE: The memo program and clients are now maintained at
[solana-program/memo](https://github.com/solana-program/memo).

# Memo Program

A simple program that validates a string of UTF-8 encoded characters and logs it
in the transaction log. The program also verifies that any accounts provided are
signers of the transaction, and if so, logs their addresses. It can be used to
record a string on-chain, stored in the instruction data of a successful
transaction, and optionally verify the originator.

Full documentation is available at https://spl.solana.com/memo

## Audit

The repository [README](https://github.com/solana-labs/solana-program-library#audits)
contains information about program audits.
