"""Stake Program Constants."""

from solana.publickey import PublicKey

STAKE_PROGRAM_ID: PublicKey = PublicKey("Stake11111111111111111111111111111111111111")
"""Public key that identifies the Stake program."""

SYSVAR_STAKE_CONFIG_ID: PublicKey = PublicKey("StakeConfig11111111111111111111111111111111")
"""Public key that identifies the Stake config sysvar."""

STAKE_LEN: int = 200
"""Size of stake account."""
