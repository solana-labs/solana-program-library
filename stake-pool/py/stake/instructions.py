"""Stake Program Instructions."""

from enum import IntEnum
from typing import NamedTuple

from construct import Switch  # type: ignore
from construct import Int32ul, Pass  # type: ignore
from construct import Struct

from solana.publickey import PublicKey
from solana.sysvar import SYSVAR_RENT_PUBKEY
from solana.transaction import AccountMeta, TransactionInstruction

from stake.constants import STAKE_PROGRAM_ID
from stake.state import AUTHORIZED_LAYOUT, LOCKUP_LAYOUT, Authorized, Lockup


class InitializeParams(NamedTuple):
    """Initialize stake transaction params."""

    stake: PublicKey
    """`[w]` Uninitialized stake account."""
    authorized: Authorized
    """Information about the staker and withdrawer keys."""
    lockup: Lockup
    """Stake lockup, if any."""


class InstructionType(IntEnum):
    """Stake Instruction Types."""

    INITIALIZE = 0
    AUTHORIZE = 1
    DELEGATE_STAKE = 2
    SPLIT = 3
    WITHDRAW = 4
    DEACTIVATE = 5
    SET_LOCKUP = 6
    MERGE = 7
    AUTHORIZE_WITH_SEED = 8
    INITIALIZE_CHECKED = 9
    AUTHORIZED_CHECKED = 10
    AUTHORIZED_CHECKED_WITH_SEED = 11
    SET_LOCKUP_CHECKED = 12


INITIALIZE_LAYOUT = Struct(
    "authorized" / AUTHORIZED_LAYOUT,
    "lockup" / LOCKUP_LAYOUT,
)


INSTRUCTIONS_LAYOUT = Struct(
    "instruction_type" / Int32ul,
    "args"
    / Switch(
        lambda this: this.instruction_type,
        {
            InstructionType.INITIALIZE: INITIALIZE_LAYOUT,
            InstructionType.AUTHORIZE: Pass,
            InstructionType.DELEGATE_STAKE: Pass,
            InstructionType.SPLIT: Pass,
            InstructionType.WITHDRAW: Pass,
            InstructionType.DEACTIVATE: Pass,
            InstructionType.SET_LOCKUP: Pass,
            InstructionType.MERGE: Pass,
            InstructionType.AUTHORIZE_WITH_SEED: Pass,
            InstructionType.INITIALIZE_CHECKED: Pass,
            InstructionType.AUTHORIZED_CHECKED: Pass,
            InstructionType.AUTHORIZED_CHECKED_WITH_SEED: Pass,
            InstructionType.SET_LOCKUP_CHECKED: Pass,
        },
    ),
)


def initialize(params: InitializeParams) -> TransactionInstruction:
    """Creates a transaction instruction to initialize a new stake."""
    data = INSTRUCTIONS_LAYOUT.build(
        dict(
            instruction_type=InstructionType.INITIALIZE,
            args=dict(
                authorized=params.authorized.as_bytes_dict(),
                lockup=params.lockup.as_bytes_dict(),
            ),
        )
    )
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=SYSVAR_RENT_PUBKEY, is_signer=False, is_writable=False),
        ],
        program_id=STAKE_PROGRAM_ID,
        data=data,
    )
