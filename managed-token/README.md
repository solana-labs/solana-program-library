# Managed Token

On-chain program for "managed tokens", SPL tokens that are perpetually frozen,
and must be used through this program, which will thaw the account, perform an
instruction, and re-freeze the account.

## Audit

The repository [README](https://github.com/solana-labs/solana-program-library#audits)
contains information about program audits.
