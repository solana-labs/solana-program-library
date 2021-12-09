"""SPL Stake Pool Constants."""

from typing import Tuple

from solana.publickey import PublicKey

STAKE_POOL_PROGRAM_ID: PublicKey = PublicKey("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")
"""Public key that identifies the SPL Stake Pool program."""

MAX_VALIDATORS_TO_UPDATE: int = 5
"""Maximum number of validators to update during UpdateValidatorListBalance."""


def find_deposit_authority_program_address(
    program_id: PublicKey,
    stake_pool_address: PublicKey,
) -> Tuple[PublicKey, int]:
    """Generates the deposit authority program address for the stake pool"""
    return PublicKey.find_program_address(
        [bytes(stake_pool_address), AUTHORITY_DEPOSIT],
        program_id,
    )


def find_withdraw_authority_program_address(
    program_id: PublicKey,
    stake_pool_address: PublicKey,
) -> Tuple[PublicKey, int]:
    """Generates the withdraw authority program address for the stake pool"""
    return PublicKey.find_program_address(
        [bytes(stake_pool_address), AUTHORITY_WITHDRAW],
        program_id,
    )


def find_stake_program_address(
    program_id: PublicKey,
    vote_account_address: PublicKey,
    stake_pool_address: PublicKey,
) -> Tuple[PublicKey, int]:
    """Generates the stake program address for a validator's vote account"""
    return PublicKey.find_program_address(
        [
            bytes(vote_account_address),
            bytes(stake_pool_address),
        ],
        program_id,
    )


def find_transient_stake_program_address(
    program_id: PublicKey,
    vote_account_address: PublicKey,
    stake_pool_address: PublicKey,
    seed: int,
) -> Tuple[PublicKey, int]:
    """Generates the stake program address for a validator's vote account"""
    return PublicKey.find_program_address(
        [
            TRANSIENT_STAKE_SEED_PREFIX,
            bytes(vote_account_address),
            bytes(stake_pool_address),
            seed.to_bytes(8, 'little'),
        ],
        program_id,
    )


AUTHORITY_DEPOSIT = b"deposit"
"""Seed used to derive the default stake pool deposit authority."""
AUTHORITY_WITHDRAW = b"withdraw"
"""Seed used to derive the stake pool withdraw authority."""
TRANSIENT_STAKE_SEED_PREFIX = b"transient"
"""Seed used to derive transient stake accounts."""
