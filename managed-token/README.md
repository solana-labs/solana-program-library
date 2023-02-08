# Managed Token

On-chain program for "managed tokens", SPL tokens that are perpetually frozen,
and must be used through this program, which will thaw the account, perform an
instruction, and re-freeze the account.

## Note

This code is unaudited. Use at your own risk.
