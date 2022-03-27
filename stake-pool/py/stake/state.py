"""Stake State."""

from enum import IntEnum
from typing import NamedTuple, Dict
from construct import Container, Struct, Float64l, Int32ul, Int64ul  # type: ignore

from solana.publickey import PublicKey
from solana.utils.helpers import decode_byte_string
from solana._layouts.shared import PUBLIC_KEY_LAYOUT


class Lockup(NamedTuple):
    """Lockup for a stake account."""
    unix_timestamp: int
    epoch: int
    custodian: PublicKey

    @classmethod
    def decode_container(cls, container: Container):
        return Lockup(
            unix_timestamp=container['unix_timestamp'],
            epoch=container['epoch'],
            custodian=PublicKey(container['custodian']),
        )

    def as_bytes_dict(self) -> Dict:
        self_dict = self._asdict()
        self_dict['custodian'] = bytes(self_dict['custodian'])
        return self_dict


class Authorized(NamedTuple):
    """Define who is authorized to change a stake."""
    staker: PublicKey
    withdrawer: PublicKey

    def as_bytes_dict(self) -> Dict:
        return {
            'staker': bytes(self.staker),
            'withdrawer': bytes(self.withdrawer),
        }


class StakeAuthorize(IntEnum):
    """Stake Authorization Types."""
    STAKER = 0
    WITHDRAWER = 1


class StakeStateType(IntEnum):
    """Stake State Types."""
    UNINITIALIZED = 0
    INITIALIZED = 1
    STAKE = 2
    REWARDS_POOL = 3


class StakeState(NamedTuple):
    state_type: StakeStateType
    state: Container

    """Stake state."""
    @classmethod
    def decode(cls, data: str, encoding: str):
        data_bytes = decode_byte_string(data, encoding)
        parsed = STAKE_STATE_LAYOUT.parse(data_bytes)
        return StakeState(
            state_type=parsed['state_type'],
            state=parsed['state'],
        )


LOCKUP_LAYOUT = Struct(
    "unix_timestamp" / Int64ul,
    "epoch" / Int64ul,
    "custodian" / PUBLIC_KEY_LAYOUT,
)


AUTHORIZED_LAYOUT = Struct(
    "staker" / PUBLIC_KEY_LAYOUT,
    "withdrawer" / PUBLIC_KEY_LAYOUT,
)

META_LAYOUT = Struct(
    "rent_exempt_reserve" / Int64ul,
    "authorized" / AUTHORIZED_LAYOUT,
    "lockup" / LOCKUP_LAYOUT,
)

META_LAYOUT = Struct(
    "rent_exempt_reserve" / Int64ul,
    "authorized" / AUTHORIZED_LAYOUT,
    "lockup" / LOCKUP_LAYOUT,
)

DELEGATION_LAYOUT = Struct(
    "voter_pubkey" / PUBLIC_KEY_LAYOUT,
    "stake" / Int64ul,
    "activation_epoch" / Int64ul,
    "deactivation_epoch" / Int64ul,
    "warmup_cooldown_rate" / Float64l,
)

STAKE_LAYOUT = Struct(
    "delegation" / DELEGATION_LAYOUT,
    "credits_observed" / Int64ul,
)

STAKE_AND_META_LAYOUT = Struct(
    "meta" / META_LAYOUT,
    "stake" / STAKE_LAYOUT,
)

STAKE_STATE_LAYOUT = Struct(
    "state_type" / Int32ul,
    "state" / STAKE_AND_META_LAYOUT,
    # NOTE: This can be done better, but was mainly needed for testing. Ideally,
    # we would have something like:
    #
    # Switch(
    #     lambda this: this.state,
    #     {
    #         StakeStateType.UNINITIALIZED: Pass,
    #         StakeStateType.INITIALIZED: META_LAYOUT,
    #         StakeStateType.STAKE: STAKE_AND_META_LAYOUT,
    #     }
    # ),
    #
    # Unfortunately, it didn't work.
)
