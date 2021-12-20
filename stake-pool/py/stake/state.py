"""Stake State."""

from enum import IntEnum
from typing import NamedTuple, Dict
from construct import Container, Struct, Int64ul  # type: ignore

from solana.publickey import PublicKey
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


LOCKUP_LAYOUT = Struct(
    "unix_timestamp" / Int64ul,
    "epoch" / Int64ul,
    "custodian" / PUBLIC_KEY_LAYOUT,
)


AUTHORIZED_LAYOUT = Struct(
    "staker" / PUBLIC_KEY_LAYOUT,
    "withdrawer" / PUBLIC_KEY_LAYOUT,
)
