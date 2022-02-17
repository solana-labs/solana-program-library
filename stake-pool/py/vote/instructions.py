"""Vote Program Instructions."""

from enum import IntEnum
from typing import NamedTuple

from construct import Struct, Switch, Int8ul, Int32ul, Pass  # type: ignore

from solana.publickey import PublicKey
from solana.sysvar import SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY
from solana.transaction import AccountMeta, TransactionInstruction
from solana._layouts.shared import PUBLIC_KEY_LAYOUT

from vote.constants import VOTE_PROGRAM_ID


class InitializeParams(NamedTuple):
    """Initialize vote account params."""

    vote: PublicKey
    """`[w]` Uninitialized vote account"""
    rent_sysvar: PublicKey
    """`[]` Rent sysvar."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    node: PublicKey
    """`[s]` New validator identity."""

    authorized_voter: PublicKey
    """The authorized voter for this vote account."""
    authorized_withdrawer: PublicKey
    """The authorized withdrawer for this vote account."""
    commission: int
    """Commission, represented as a percentage"""


class InstructionType(IntEnum):
    """Vote Instruction Types."""

    INITIALIZE = 0
    AUTHORIZE = 1
    VOTE = 2
    WITHDRAW = 3
    UPDATE_VALIDATOR_IDENTITY = 4
    UPDATE_COMMISSION = 5
    VOTE_SWITCH = 6
    AUTHORIZE_CHECKED = 7


INITIALIZE_LAYOUT = Struct(
    "node" / PUBLIC_KEY_LAYOUT,
    "authorized_voter" / PUBLIC_KEY_LAYOUT,
    "authorized_withdrawer" / PUBLIC_KEY_LAYOUT,
    "commission" / Int8ul,
)

INSTRUCTIONS_LAYOUT = Struct(
    "instruction_type" / Int32ul,
    "args"
    / Switch(
        lambda this: this.instruction_type,
        {
            InstructionType.INITIALIZE: INITIALIZE_LAYOUT,
            InstructionType.AUTHORIZE: Pass,  # TODO
            InstructionType.VOTE: Pass,  # TODO
            InstructionType.WITHDRAW: Pass,  # TODO
            InstructionType.UPDATE_VALIDATOR_IDENTITY: Pass,  # TODO
            InstructionType.UPDATE_COMMISSION: Pass,  # TODO
            InstructionType.VOTE_SWITCH: Pass,  # TODO
            InstructionType.AUTHORIZE_CHECKED: Pass,  # TODO
        },
    ),
)


def initialize(params: InitializeParams) -> TransactionInstruction:
    """Creates a transaction instruction to initialize a new stake."""
    data = INSTRUCTIONS_LAYOUT.build(
        dict(
            instruction_type=InstructionType.INITIALIZE,
            args=dict(
                node=bytes(params.node),
                authorized_voter=bytes(params.authorized_voter),
                authorized_withdrawer=bytes(params.authorized_withdrawer),
                commission=params.commission,
            ),
        )
    )
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.vote, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.rent_sysvar or SYSVAR_RENT_PUBKEY, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.clock_sysvar or SYSVAR_CLOCK_PUBKEY, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.node, is_signer=True, is_writable=False),
        ],
        program_id=VOTE_PROGRAM_ID,
        data=data,
    )
