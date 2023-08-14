## Solana Program Library Single-Validator Stake Pool

The single-validator stake pool program is an upcoming SPL program that enables liquid staking with zero fees, no counterparty, and 100% capital efficiency.

The program defines a canonical pool for every vote account, which can be initialized permissionlessly, and mints tokens in exchange for stake delegated to its designated validator.

The program is a stripped-down adaptation of the existing multi-validator stake pool program, with approximately 80% less code, to minimize execution risk.

On launch, users will only be able to deposit and withdraw active stake, but pending future stake program development, we hope to support instant sol deposits and withdrawals as well.
