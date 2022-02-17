"""Stake Program Instructions."""

from enum import IntEnum
from typing import NamedTuple

from construct import Switch  # type: ignore
from construct import Int32ul, Pass  # type: ignore
from construct import Struct

from solana._layouts.shared import PUBLIC_KEY_LAYOUT
from solana.publickey import PublicKey
from solana.sysvar import SYSVAR_RENT_PUBKEY
from solana.transaction import AccountMeta, TransactionInstruction

from stake.constants import STAKE_PROGRAM_ID
from stake.state import AUTHORIZED_LAYOUT, LOCKUP_LAYOUT, Authorized, Lockup, StakeAuthorize


class InitializeParams(NamedTuple):
    """Initialize stake transaction params."""

    stake: PublicKey
    """`[w]` Uninitialized stake account."""
    authorized: Authorized
    """Information about the staker and withdrawer keys."""
    lockup: Lockup
    """Stake lockup, if any."""


class DelegateStakeParams(NamedTuple):
    """Initialize stake transaction params."""

    stake: PublicKey
    """`[w]` Uninitialized stake account."""
    vote: PublicKey
    """`[]` Vote account to which this stake will be delegated."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    stake_history_sysvar: PublicKey
    """`[]` Stake history sysvar that carries stake warmup/cooldown history."""
    stake_config_id: PublicKey
    """`[]` Address of config account that carries stake config."""
    staker: PublicKey
    """`[s]` Stake authority."""


class AuthorizeParams(NamedTuple):
    """Authorize stake transaction params."""

    stake: PublicKey
    """`[w]` Initialized stake account to modify."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    authority: PublicKey
    """`[s]` Current stake authority."""

    # Params
    new_authority: PublicKey
    """New authority's public key."""
    stake_authorize: StakeAuthorize
    """Type of authority to modify, staker or withdrawer."""


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


AUTHORIZE_LAYOUT = Struct(
    "new_authority" / PUBLIC_KEY_LAYOUT,
    "stake_authorize" / Int32ul,
)


INSTRUCTIONS_LAYOUT = Struct(
    "instruction_type" / Int32ul,
    "args"
    / Switch(
        lambda this: this.instruction_type,
        {
            InstructionType.INITIALIZE: INITIALIZE_LAYOUT,
            InstructionType.AUTHORIZE: AUTHORIZE_LAYOUT,
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
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=SYSVAR_RENT_PUBKEY, is_signer=False, is_writable=False),
        ],
        program_id=STAKE_PROGRAM_ID,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.INITIALIZE,
                args=dict(
                    authorized=params.authorized.as_bytes_dict(),
                    lockup=params.lockup.as_bytes_dict(),
                ),
            )
        )
    )


def delegate_stake(params: DelegateStakeParams) -> TransactionInstruction:
    """Creates an instruction to delegate a stake account."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.vote, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_history_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_config_id, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.staker, is_signer=True, is_writable=False),
        ],
        program_id=STAKE_PROGRAM_ID,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.DELEGATE_STAKE,
                args=None,
            )
        )
    )


def authorize(params: AuthorizeParams) -> TransactionInstruction:
    """Creates an instruction to change the authority on a stake account."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.authority, is_signer=True, is_writable=False),
        ],
        program_id=STAKE_PROGRAM_ID,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.AUTHORIZE,
                args={
                    'new_authority': bytes(params.new_authority),
                    'stake_authorize': params.stake_authorize,
                },
            )
        )
    )
