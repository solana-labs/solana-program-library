"""SPL Stake Pool State."""

from enum import IntEnum
from typing import List, NamedTuple, Optional
from construct import Container, Struct, Switch, Int8ul, Int32ul, Int64ul, Pass  # type: ignore

from solana.publickey import PublicKey
from solana.utils.helpers import decode_byte_string
from solana._layouts.shared import PUBLIC_KEY_LAYOUT
from stake.state import Lockup, LOCKUP_LAYOUT


def decode_optional_publickey(container: Container) -> Optional[PublicKey]:
    if container:
        return PublicKey(container)
    else:
        return None


class Fee(NamedTuple):
    """Fee assessed by the stake pool, expressed as numerator / denominator."""
    numerator: int
    denominator: int

    @classmethod
    def decode_container(cls, container: Container):
        return Fee(
            numerator=container['numerator'],
            denominator=container['denominator'],
        )

    @classmethod
    def decode_optional_container(cls, container: Container):
        if container:
            return cls.decode_container(container)
        else:
            return None


class StakePool(NamedTuple):
    """Stake pool and all its data."""
    manager: PublicKey
    staker: PublicKey
    stake_deposit_authority: PublicKey
    stake_withdraw_bump_seed: int
    validator_list: PublicKey
    reserve_stake: PublicKey
    pool_mint: PublicKey
    manager_fee_account: PublicKey
    token_program_id: PublicKey
    total_lamports: int
    pool_token_supply: int
    last_update_epoch: int
    lockup: Lockup
    epoch_fee: Fee
    next_epoch_fee: Optional[Fee]
    preferred_deposit_validator: Optional[PublicKey]
    preferred_withdraw_validator: Optional[PublicKey]
    stake_deposit_fee: Fee
    stake_withdrawal_fee: Fee
    next_stake_withdrawal_fee: Optional[Fee]
    stake_referral_fee: int
    sol_deposit_authority: Optional[PublicKey]
    sol_deposit_fee: Fee
    sol_referral_fee: int
    sol_withdraw_authority: Optional[PublicKey]
    sol_withdrawal_fee: Fee
    next_sol_withdrawal_fee: Optional[Fee]
    last_epoch_pool_token_supply: int
    last_epoch_total_lamports: int

    @classmethod
    def decode(cls, data: str, encoding: str):
        data_bytes = decode_byte_string(data, encoding)
        parsed = DECODE_STAKE_POOL_LAYOUT.parse(data_bytes)
        return StakePool(
            manager=PublicKey(parsed['manager']),
            staker=PublicKey(parsed['staker']),
            stake_deposit_authority=PublicKey(parsed['stake_deposit_authority']),
            stake_withdraw_bump_seed=parsed['stake_withdraw_bump_seed'],
            validator_list=PublicKey(parsed['validator_list']),
            reserve_stake=PublicKey(parsed['reserve_stake']),
            pool_mint=PublicKey(parsed['pool_mint']),
            manager_fee_account=PublicKey(parsed['manager_fee_account']),
            token_program_id=PublicKey(parsed['token_program_id']),
            total_lamports=parsed['total_lamports'],
            pool_token_supply=parsed['pool_token_supply'],
            last_update_epoch=parsed['last_update_epoch'],
            lockup=Lockup.decode_container(parsed['lockup']),
            epoch_fee=Fee.decode_container(parsed['epoch_fee']),
            next_epoch_fee=Fee.decode_optional_container(parsed['next_epoch_fee']),
            preferred_deposit_validator=decode_optional_publickey(parsed['preferred_deposit_validator']),
            preferred_withdraw_validator=decode_optional_publickey(parsed['preferred_withdraw_validator']),
            stake_deposit_fee=Fee.decode_container(parsed['stake_deposit_fee']),
            stake_withdrawal_fee=Fee.decode_container(parsed['stake_withdrawal_fee']),
            next_stake_withdrawal_fee=Fee.decode_optional_container(parsed['next_stake_withdrawal_fee']),
            stake_referral_fee=parsed['stake_referral_fee'],
            sol_deposit_authority=decode_optional_publickey(parsed['sol_deposit_authority']),
            sol_deposit_fee=Fee.decode_container(parsed['sol_deposit_fee']),
            sol_referral_fee=parsed['sol_referral_fee'],
            sol_withdraw_authority=decode_optional_publickey(parsed['sol_withdraw_authority']),
            sol_withdrawal_fee=Fee.decode_container(parsed['sol_withdrawal_fee']),
            next_sol_withdrawal_fee=Fee.decode_optional_container(parsed['next_sol_withdrawal_fee']),
            last_epoch_pool_token_supply=parsed['last_epoch_pool_token_supply'],
            last_epoch_total_lamports=parsed['last_epoch_total_lamports'],
        )


class StakeStatus(IntEnum):
    """Specifies the status of a stake on a validator in a stake pool."""

    ACTIVE = 0
    """Stake is active and normal."""
    DEACTIVATING_TRANSIENT = 1
    """Stake has been removed, but a deactivating transient stake still exists."""
    READY_FOR_REMOVAL = 2
    """No more validator stake accounts exist, entry ready for removal."""


class ValidatorStakeInfo(NamedTuple):
    active_stake_lamports: int
    """Amount of active stake delegated to this validator."""

    transient_stake_lamports: int
    """Amount of transient stake delegated to this validator."""

    last_update_epoch: int
    """Last epoch the active and transient stake lamports fields were updated."""

    transient_seed_suffix_start: int
    """Start of the validator transient account seed suffixes."""

    transient_seed_suffix_end: int
    """End of the validator transient account seed suffixes."""

    status: StakeStatus
    """Status of the validator stake account."""

    vote_account_address: PublicKey
    """Validator vote account address."""

    @classmethod
    def decode_container(cls, container: Container):
        return ValidatorStakeInfo(
            active_stake_lamports=container['active_stake_lamports'],
            transient_stake_lamports=container['transient_stake_lamports'],
            last_update_epoch=container['last_update_epoch'],
            transient_seed_suffix_start=container['transient_seed_suffix_start'],
            transient_seed_suffix_end=container['transient_seed_suffix_end'],
            status=container['status'],
            vote_account_address=PublicKey(container['vote_account_address']),
        )


class ValidatorList(NamedTuple):
    """List of validators and amount staked, associated to a stake pool."""

    max_validators: int
    """Maximum number of validators possible in the list."""

    validators: List[ValidatorStakeInfo]
    """Info for each validator in the stake pool."""

    @staticmethod
    def calculate_validator_list_size(max_validators: int) -> int:
        layout = VALIDATOR_LIST_LAYOUT + VALIDATOR_INFO_LAYOUT[max_validators]
        return layout.sizeof()

    @classmethod
    def decode(cls, data: str, encoding: str):
        data_bytes = decode_byte_string(data, encoding)
        parsed = DECODE_VALIDATOR_LIST_LAYOUT.parse(data_bytes)
        print(parsed)
        return ValidatorList(
            max_validators=parsed['max_validators'],
            validators=[ValidatorStakeInfo.decode_container(container) for container in parsed['validators']],
        )


FEE_LAYOUT = Struct(
    "denominator" / Int64ul,
    "numerator" / Int64ul,
)

STAKE_POOL_LAYOUT = Struct(
    "account_type" / Int8ul,
    "manager" / PUBLIC_KEY_LAYOUT,
    "staker" / PUBLIC_KEY_LAYOUT,
    "stake_deposit_authority" / PUBLIC_KEY_LAYOUT,
    "stake_withdraw_bump_seed" / Int8ul,
    "validator_list" / PUBLIC_KEY_LAYOUT,
    "reserve_stake" / PUBLIC_KEY_LAYOUT,
    "pool_mint" / PUBLIC_KEY_LAYOUT,
    "manager_fee_account" / PUBLIC_KEY_LAYOUT,
    "token_program_id" / PUBLIC_KEY_LAYOUT,
    "total_lamports" / Int64ul,
    "pool_token_supply" / Int64ul,
    "last_update_epoch" / Int64ul,
    "lockup" / LOCKUP_LAYOUT,
    "epoch_fee" / FEE_LAYOUT,
    "next_epoch_fee_option" / Int8ul,
    "next_epoch_fee" / FEE_LAYOUT,
    "preferred_deposit_validator_option" / Int8ul,
    "preferred_deposit_validator" / PUBLIC_KEY_LAYOUT,
    "preferred_withdraw_validator_option" / Int8ul,
    "preferred_withdraw_validator" / PUBLIC_KEY_LAYOUT,
    "stake_deposit_fee" / FEE_LAYOUT,
    "stake_withdrawal_fee" / FEE_LAYOUT,
    "next_stake_withdrawal_fee_option" / Int8ul,
    "next_stake_withdrawal_fee" / FEE_LAYOUT,
    "stake_referral_fee" / Int8ul,
    "sol_deposit_authority_option" / Int8ul,
    "sol_deposit_authority" / PUBLIC_KEY_LAYOUT,
    "sol_deposit_fee" / FEE_LAYOUT,
    "sol_referral_fee" / Int8ul,
    "sol_withdraw_authority_option" / Int8ul,
    "sol_withdraw_authority" / PUBLIC_KEY_LAYOUT,
    "sol_withdrawal_fee" / FEE_LAYOUT,
    "next_sol_withdrawal_fee_option" / Int8ul,
    "next_sol_withdrawal_fee" / FEE_LAYOUT,
    "last_epoch_pool_token_supply" / Int64ul,
    "last_epoch_total_lamports" / Int64ul,
)

DECODE_STAKE_POOL_LAYOUT = Struct(
    "account_type" / Int8ul,
    "manager" / PUBLIC_KEY_LAYOUT,
    "staker" / PUBLIC_KEY_LAYOUT,
    "stake_deposit_authority" / PUBLIC_KEY_LAYOUT,
    "stake_withdraw_bump_seed" / Int8ul,
    "validator_list" / PUBLIC_KEY_LAYOUT,
    "reserve_stake" / PUBLIC_KEY_LAYOUT,
    "pool_mint" / PUBLIC_KEY_LAYOUT,
    "manager_fee_account" / PUBLIC_KEY_LAYOUT,
    "token_program_id" / PUBLIC_KEY_LAYOUT,
    "total_lamports" / Int64ul,
    "pool_token_supply" / Int64ul,
    "last_update_epoch" / Int64ul,
    "lockup" / LOCKUP_LAYOUT,
    "epoch_fee" / FEE_LAYOUT,
    "next_epoch_fee_option" / Int8ul,
    "next_epoch_fee" / Switch(
        lambda this: this.next_epoch_fee_option,
        {
            0: Pass,
            1: FEE_LAYOUT,
        }),
    "preferred_deposit_validator_option" / Int8ul,
    "preferred_deposit_validator" / Switch(
        lambda this: this.preferred_deposit_validator_option,
        {
            0: Pass,
            1: PUBLIC_KEY_LAYOUT,
        }),
    "preferred_withdraw_validator_option" / Int8ul,
    "preferred_withdraw_validator" / Switch(
        lambda this: this.preferred_withdraw_validator_option,
        {
            0: Pass,
            1: PUBLIC_KEY_LAYOUT,
        }),
    "stake_deposit_fee" / FEE_LAYOUT,
    "stake_withdrawal_fee" / FEE_LAYOUT,
    "next_stake_withdrawal_fee_option" / Int8ul,
    "next_stake_withdrawal_fee" / Switch(
        lambda this: this.next_stake_withdrawal_fee_option,
        {
            0: Pass,
            1: FEE_LAYOUT,
        }),
    "stake_referral_fee" / Int8ul,
    "sol_deposit_authority_option" / Int8ul,
    "sol_deposit_authority" / Switch(
        lambda this: this.sol_deposit_authority_option,
        {
            0: Pass,
            1: PUBLIC_KEY_LAYOUT,
        }),
    "sol_deposit_fee" / FEE_LAYOUT,
    "sol_referral_fee" / Int8ul,
    "sol_withdraw_authority_option" / Int8ul,
    "sol_withdraw_authority" / Switch(
        lambda this: this.sol_withdraw_authority_option,
        {
            0: Pass,
            1: PUBLIC_KEY_LAYOUT,
        }),
    "sol_withdrawal_fee" / FEE_LAYOUT,
    "next_sol_withdrawal_fee_option" / Int8ul,
    "next_sol_withdrawal_fee" / Switch(
        lambda this: this.next_sol_withdrawal_fee_option,
        {
            0: Pass,
            1: FEE_LAYOUT,
        }),
    "last_epoch_pool_token_supply" / Int64ul,
    "last_epoch_total_lamports" / Int64ul,
)

VALIDATOR_INFO_LAYOUT = Struct(
    "active_stake_lamports" / Int64ul,
    "transient_stake_lamports" / Int64ul,
    "last_update_epoch" / Int64ul,
    "transient_seed_suffix_start" / Int64ul,
    "transient_seed_suffix_end" / Int64ul,
    "status" / Int8ul,
    "vote_account_address" / PUBLIC_KEY_LAYOUT,
)

VALIDATOR_LIST_LAYOUT = Struct(
    "account_type" / Int8ul,
    "max_validators" / Int32ul,
    "validators_len" / Int32ul,
)

DECODE_VALIDATOR_LIST_LAYOUT = Struct(
    "account_type" / Int8ul,
    "max_validators" / Int32ul,
    "validators_len" / Int32ul,
    "validators" / VALIDATOR_INFO_LAYOUT[lambda this: this.validators_len],
)
