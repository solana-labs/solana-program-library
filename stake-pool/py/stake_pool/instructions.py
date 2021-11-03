"""SPL Stake Pool Instructions."""

from enum import IntEnum
from typing import List, NamedTuple, Optional
from construct import Struct, Switch, Int8ul, Int32ul, Int64ul, Pass  # type: ignore

from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction
from spl.token.constants import TOKEN_PROGRAM_ID

from stake_pool.state import Fee, FEE_LAYOUT


class PreferredValidatorType(IntEnum):
    """Specifies the validator type for SetPreferredValidator instruction."""

    DEPOSIT = 0
    """Specifies the preferred deposit validator."""
    WITHDRAW = 1
    """Specifies the preferred withdraw validator."""


class FundingType(IntEnum):
    """Defines which authority to update in the `SetFundingAuthority` instruction."""

    STAKE_DEPOSIT = 0
    """Sets the stake deposit authority."""
    SOL_DEPOSIT = 1
    """Sets the SOL deposit authority."""
    SOL_WITHDRAW = 2
    """Sets the SOL withdraw authority."""


class InitializeParams(NamedTuple):
    """Initialize token mint transaction params."""

    # Accounts
    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """[w] Stake Pool account to initialize."""
    manager: PublicKey
    """[s] Manager for new stake pool."""
    staker: PublicKey
    """[] Staker for the new stake pool."""
    validator_list: PublicKey
    """[w] Uninitialized validator list account for the new stake pool."""
    reserve_stake: PublicKey
    """[] Reserve stake account."""
    pool_mint: PublicKey
    """[] Pool token mint account."""
    manager_fee_account: PublicKey
    """[] Manager's fee account"""
    token_program_id: PublicKey
    """[] SPL Token program id."""

    # Params
    epoch_fee: Fee
    """Fee assessed as percentage of rewards."""
    withdrawal_fee: Fee
    """Fee charged per withdrawal."""
    deposit_fee: Fee
    """Fee charged per deposit."""
    referral_fee: int
    """Percentage [0-100] of deposit fee that goes to referrer."""
    max_validators: int
    """Maximum number of possible validators in the pool."""

    # Optional
    deposit_authority: Optional[PublicKey] = None
    """[] Optional deposit authority that must sign all deposits."""


class AddValidatorToPoolParams(NamedTuple):
    """(Staker only) Adds stake account delegated to validator to the pool's list of managed validators."""

    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool."""
    staker: PublicKey
    """`[s]` Staker."""
    funding_account: PublicKey
    """`[ws]` Funding account (must be a system account)."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""
    validator_stake: PublicKey
    """`[w]` Stake account to add to the pool."""
    validator_vote: PublicKey
    """`[]` Validator this stake account will be delegated to."""
    rent_sysvar: PublicKey
    """`[]` Rent sysvar."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    stake_history_sysvar: PublicKey
    """'[]' Stake history sysvar."""
    stake_config_sysvar: PublicKey
    """'[]' Stake config sysvar."""
    system_program_id: PublicKey
    """`[]` System program."""
    stake_program_id: PublicKey
    """`[]` Stake program."""


class RemoveValidatorFromPoolParams(NamedTuple):
    """(Staker only) Removes validator from the pool."""

    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool."""
    staker: PublicKey
    """`[s]` Staker."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    new_stake_authority: PublicKey
    """`[]` New stake / withdraw authority on the split stake account."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""
    validator_stake: PublicKey
    """`[w]` Stake account to remove from the pool."""
    transient_stake: PublicKey
    """`[]` Transient stake account, to check that there's no activation ongoing."""
    destination_stake: PublicKey
    """`[w]` Destination stake account, to receive the minimum SOL from the validator stake account."""
    clock_sysvar: PublicKey
    """'[]' Stake config sysvar."""
    stake_program_id: PublicKey
    """`[]` Stake program."""


class DecreaseValidatorStakeParams(NamedTuple):
    """(Staker only) Decrease active stake on a validator, eventually moving it to the reserve"""

    # Accounts
    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[]` Stake pool."""
    staker: PublicKey
    """`[s]` Staker."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""
    validator_stake: PublicKey
    """`[w]` Canonical stake to split from."""
    transient_stake: PublicKey
    """`[w]` Transient stake account to receive split."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    rent_sysvar: PublicKey
    """`[]` Rent sysvar."""
    system_program_id: PublicKey
    """`[]` System program."""
    stake_program_id: PublicKey
    """`[]` Stake program."""

    # Params
    lamports: int
    """Amount of lamports to split into the transient stake account."""
    transient_stake_seed: int
    """Seed to used to create the transient stake account."""


class IncreaseValidatorStakeParams(NamedTuple):
    """(Staker only) Increase stake on a validator from the reserve account."""

    # Accounts
    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[]` Stake pool."""
    staker: PublicKey
    """`[s]` Staker."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""
    reserve_stake: PublicKey
    """`[w]` Stake pool's reserve."""
    transient_stake: PublicKey
    """`[w]` Transient stake account to receive split."""
    validator_vote: PublicKey
    """`[]` Validator vote account to delegate to."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    rent_sysvar: PublicKey
    """`[]` Rent sysvar."""
    stake_history_sysvar: PublicKey
    """'[]' Stake history sysvar."""
    stake_config_sysvar: PublicKey
    """'[]' Stake config sysvar."""
    system_program_id: PublicKey
    """`[]` System program."""
    stake_program_id: PublicKey
    """`[]` Stake program."""

    # Params
    lamports: int
    """Amount of lamports to split into the transient stake account."""
    transient_stake_seed: int
    """Seed to used to create the transient stake account."""


class SetPreferredValidatorParams(NamedTuple):
    pass


class UpdateValidatorListBalanceParams(NamedTuple):
    """Updates balances of validator and transient stake accounts in the pool."""

    # Accounts
    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[]` Stake pool."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""
    reserve_stake: PublicKey
    """`[w]` Stake pool's reserve."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    stake_history_sysvar: PublicKey
    """'[]' Stake history sysvar."""
    stake_program_id: PublicKey
    """`[]` Stake program."""
    validator_and_transient_stake_pairs: List[PublicKey]
    """[] N pairs of validator and transient stake accounts"""

    # Params
    start_index: int
    """Index to start updating on the validator list."""
    no_merge: bool
    """If true, don't try merging transient stake accounts."""


class UpdateStakePoolBalanceParams(NamedTuple):
    """Updates total pool balance based on balances in the reserve and validator list."""

    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""
    reserve_stake: PublicKey
    """`[w]` Stake pool's reserve."""
    manager_fee_account: PublicKey
    """`[w]` Account to receive pool fee tokens."""
    pool_mint: PublicKey
    """`[w]` Pool mint account."""
    token_program_id: PublicKey
    """`[]` Pool token program."""


class CleanupRemovedValidatorEntriesParams(NamedTuple):
    """Cleans up validator stake account entries marked as `ReadyForRemoval`"""

    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool."""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account."""


class DepositStakeParams(NamedTuple):
    pass


class WithdrawStakeParams(NamedTuple):
    pass


class SetManagerParams(NamedTuple):
    pass


class SetFeeParams(NamedTuple):
    pass


class SetStakerParams(NamedTuple):
    pass


class DepositSolParams(NamedTuple):
    """Deposit SOL directly into the pool's reserve account. The output is a "pool" token
    representing ownership into the pool. Inputs are converted to the current ratio."""

    # Accounts
    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool."""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    reserve_stake: PublicKey
    """`[w]` Stake pool's reserve."""
    funding_account: PublicKey
    """`[ws]` Funding account (must be a system account)."""
    destination_pool_account: PublicKey
    """`[w]` User account to receive pool tokens."""
    manager_fee_account: PublicKey
    """`[w]` Manager's pool token account to receive deposit fee."""
    referral_pool_account: PublicKey
    """`[w]` Referrer pool token account to receive referral fee."""
    pool_mint: PublicKey
    """`[w]` Pool token mint."""
    system_program_id: PublicKey
    """`[]` System program."""
    token_program_id: PublicKey
    """`[]` Token program."""

    # Params
    amount: int
    """Amount of SOL to deposit"""

    # Optional
    deposit_authority: Optional[PublicKey] = None
    """`[s]` (Optional) Stake pool sol deposit authority."""


class SetFundingAuthorityParams(NamedTuple):
    pass


class WithdrawSolParams(NamedTuple):
    """Withdraw SOL directly from the pool's reserve account."""

    # Accounts
    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool."""
    stake_pool_withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    user_transfer_authority: PublicKey
    """`[s]` Transfer authority for user pool token account."""
    source_pool_account: PublicKey
    """`[w]` User's pool token account to burn pool tokens."""
    reserve_stake: PublicKey
    """`[w]` Stake pool's reserve."""
    destination_system_account: PublicKey
    """`[w]` Destination system account to receive lamports from the reserve."""
    manager_fee_account: PublicKey
    """`[w]` Manager's pool token account to receive fee."""
    pool_mint: PublicKey
    """`[w]` Pool token mint."""
    clock_sysvar: PublicKey
    """`[]` Clock sysvar."""
    stake_history_sysvar: PublicKey
    """'[]' Stake history sysvar."""
    stake_program_id: PublicKey
    """`[]` Stake program."""
    token_program_id: PublicKey
    """`[]` Token program."""

    # Params
    amount: int
    """Amount of pool tokens to burn"""

    # Optional
    withdraw_authority: Optional[PublicKey] = None
    """`[s]` (Optional) Stake pool sol withdraw authority."""


class InstructionType(IntEnum):
    """Stake Pool Instruction Types."""

    INITIALIZE = 0
    ADD_VALIDATOR_TO_POOL = 1
    REMOVE_VALIDATOR_FROM_POOL = 2
    DECREASE_VALIDATOR_STAKE = 3
    INCREASE_VALIDATOR_STAKE = 4
    SET_PREFERRED_VALIDATOR = 5
    UPDATE_VALIDATOR_LIST_BALANCE = 6
    UPDATE_STAKE_POOL_BALANCE = 7
    CLEANUP_REMOVED_VALIDATOR_ENTRIES = 8
    DEPOSIT_STAKE = 9
    WITHDRAW_STAKE = 10
    SET_MANAGER = 11
    SET_FEE = 12
    SET_STAKER = 13
    DEPOSIT_SOL = 14
    SET_FUNDING_AUTHORITY = 15
    WITHDRAW_SOL = 16


INITIALIZE_LAYOUT = Struct(
    "epoch_fee" / FEE_LAYOUT,
    "withdrawal_fee" / FEE_LAYOUT,
    "deposit_fee" / FEE_LAYOUT,
    "referral_fee" / Int8ul,
    "max_validators" / Int32ul,
)

MOVE_STAKE_LAYOUT = Struct(
    "lamports" / Int64ul,
    "transient_stake_seed" / Int64ul,
)

UPDATE_VALIDATOR_LIST_BALANCE_LAYOUT = Struct(
    "start_index" / Int32ul,
    "no_merge" / Int8ul,
)

AMOUNT_LAYOUT = Struct(
    "amount" / Int64ul
)

INSTRUCTIONS_LAYOUT = Struct(
    "instruction_type" / Int8ul,
    "args"
    / Switch(
        lambda this: this.instruction_type,
        {
            InstructionType.INITIALIZE: INITIALIZE_LAYOUT,
            InstructionType.ADD_VALIDATOR_TO_POOL: Pass,
            InstructionType.REMOVE_VALIDATOR_FROM_POOL: Pass,
            InstructionType.DECREASE_VALIDATOR_STAKE: MOVE_STAKE_LAYOUT,
            InstructionType.INCREASE_VALIDATOR_STAKE: MOVE_STAKE_LAYOUT,
            InstructionType.SET_PREFERRED_VALIDATOR: Pass,  # TODO
            InstructionType.UPDATE_VALIDATOR_LIST_BALANCE: UPDATE_VALIDATOR_LIST_BALANCE_LAYOUT,
            InstructionType.UPDATE_STAKE_POOL_BALANCE: Pass,
            InstructionType.CLEANUP_REMOVED_VALIDATOR_ENTRIES: Pass,
            InstructionType.DEPOSIT_STAKE: Pass,
            InstructionType.WITHDRAW_STAKE: AMOUNT_LAYOUT,
            InstructionType.SET_MANAGER: Pass,
            InstructionType.SET_FEE: Pass,  # TODO
            InstructionType.SET_STAKER: Pass,
            InstructionType.DEPOSIT_SOL: AMOUNT_LAYOUT,
            InstructionType.SET_FUNDING_AUTHORITY: Pass,  # TODO
            InstructionType.WITHDRAW_SOL: AMOUNT_LAYOUT,
        },
    ),
)


def initialize(params: InitializeParams) -> TransactionInstruction:
    """Creates a transaction instruction to initialize a new stake pool."""

    data = INSTRUCTIONS_LAYOUT.build(
        dict(
            instruction_type=InstructionType.INITIALIZE,
            args=dict(
                epoch_fee=params.epoch_fee._asdict(),
                withdrawal_fee=params.withdrawal_fee._asdict(),
                deposit_fee=params.deposit_fee._asdict(),
                referral_fee=params.referral_fee,
                max_validators=params.max_validators
            ),
        )
    )
    keys = [
        AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.manager, is_signer=True, is_writable=False),
        AccountMeta(pubkey=params.staker, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=TOKEN_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if params.deposit_authority:
        keys.append(
            AccountMeta(pubkey=params.deposit_authority, is_signer=True, is_writable=False),
        )
    return TransactionInstruction(
        keys=keys,
        program_id=params.program_id,
        data=data,
    )
