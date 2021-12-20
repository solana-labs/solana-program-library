"""SPL Stake Pool Instructions."""

from enum import IntEnum
from typing import List, NamedTuple, Optional
from construct import Struct, Switch, Int8ul, Int32ul, Int64ul, Pass  # type: ignore

from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction
from solana.system_program import SYS_PROGRAM_ID
from solana.sysvar import SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, SYSVAR_STAKE_HISTORY_PUBKEY
from spl.token.constants import TOKEN_PROGRAM_ID

from stake.constants import STAKE_PROGRAM_ID, SYSVAR_STAKE_CONFIG_ID
from stake_pool.constants import find_stake_program_address, find_transient_stake_program_address
from stake_pool.constants import find_withdraw_authority_program_address
from stake_pool.constants import STAKE_POOL_PROGRAM_ID
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
    withdraw_authority: PublicKey
    """[] Withdraw authority for the new stake pool."""
    validator_list: PublicKey
    """[w] Uninitialized validator list account for the new stake pool."""
    reserve_stake: PublicKey
    """[] Reserve stake account."""
    pool_mint: PublicKey
    """[w] Pool token mint account."""
    manager_fee_account: PublicKey
    """[w] Manager's fee account"""
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
    """Deposits a stake account into the pool in exchange for pool tokens"""

    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool"""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account"""
    deposit_authority: PublicKey
    """`[s]/[]` Stake pool deposit authority"""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority"""
    deposit_stake: PublicKey
    """`[w]` Stake account to join the pool (stake's withdraw authority set to the stake pool deposit authority)"""
    validator_stake: PublicKey
    """`[w]` Validator stake account for the stake account to be merged with"""
    reserve_stake: PublicKey
    """`[w]` Reserve stake account, to withdraw rent exempt reserve"""
    destination_pool_account: PublicKey
    """`[w]` User account to receive pool tokens"""
    manager_fee_account: PublicKey
    """`[w]` Account to receive pool fee tokens"""
    referral_pool_account: PublicKey
    """`[w]` Account to receive a portion of pool fee tokens as referral fees"""
    pool_mint: PublicKey
    """`[w]` Pool token mint account"""
    clock_sysvar: PublicKey
    """`[]` Sysvar clock account"""
    stake_history_sysvar: PublicKey
    """`[]` Sysvar stake history account"""
    token_program_id: PublicKey
    """`[]` Pool token program id"""
    stake_program_id: PublicKey
    """`[]` Stake program id"""


class WithdrawStakeParams(NamedTuple):
    """Withdraws a stake account from the pool in exchange for pool tokens"""

    program_id: PublicKey
    """SPL Stake Pool program account."""
    stake_pool: PublicKey
    """`[w]` Stake pool"""
    validator_list: PublicKey
    """`[w]` Validator stake list storage account"""
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority"""
    validator_stake: PublicKey
    """`[w]` Validator or reserve stake account to split"""
    destination_stake: PublicKey
    """`[w]` Unitialized stake account to receive withdrawal"""
    destination_stake_authority: PublicKey
    """`[]` User account to set as a new withdraw authority"""
    source_transfer_authority: PublicKey
    """`[s]` User transfer authority, for pool token account"""
    source_pool_account: PublicKey
    """`[w]` User account with pool tokens to burn from"""
    manager_fee_account: PublicKey
    """`[w]` Account to receive pool fee tokens"""
    pool_mint: PublicKey
    """`[w]` Pool token mint account"""
    clock_sysvar: PublicKey
    """`[]` Sysvar clock account"""
    token_program_id: PublicKey
    """`[]` Pool token program id"""
    stake_program_id: PublicKey
    """`[]` Stake program id"""

    # Params
    amount: int
    """Amount of pool tokens to burn in exchange for stake"""


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
    withdraw_authority: PublicKey
    """`[]` Stake pool withdraw authority."""
    source_transfer_authority: PublicKey
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
    sol_withdraw_authority: Optional[PublicKey] = None
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
            InstructionType.SET_MANAGER: Pass,  # TODO
            InstructionType.SET_FEE: Pass,  # TODO
            InstructionType.SET_STAKER: Pass,  # TODO
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
        AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=True),
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


def add_validator_to_pool(params: AddValidatorToPoolParams) -> TransactionInstruction:
    """Creates instruction to add a validator to the pool."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.staker, is_signer=True, is_writable=False),
            AccountMeta(pubkey=params.funding_account, is_signer=True, is_writable=True),
            AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_vote, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.rent_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_history_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_config_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.system_program_id, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.ADD_VALIDATOR_TO_POOL,
                args=None
            )
        )
    )


def add_validator_to_pool_with_vote(
    program_id: PublicKey,
    stake_pool: PublicKey,
    staker: PublicKey,
    validator_list: PublicKey,
    funder: PublicKey,
    validator: PublicKey
) -> TransactionInstruction:
    """Creates instruction to add a validator based on their vote account address."""
    (withdraw_authority, seed) = find_withdraw_authority_program_address(program_id, stake_pool)
    (validator_stake, seed) = find_stake_program_address(program_id, validator, stake_pool)
    return add_validator_to_pool(
        AddValidatorToPoolParams(
            program_id=STAKE_POOL_PROGRAM_ID,
            stake_pool=stake_pool,
            staker=staker,
            funding_account=funder,
            withdraw_authority=withdraw_authority,
            validator_list=validator_list,
            validator_stake=validator_stake,
            validator_vote=validator,
            rent_sysvar=SYSVAR_RENT_PUBKEY,
            clock_sysvar=SYSVAR_CLOCK_PUBKEY,
            stake_history_sysvar=SYSVAR_STAKE_HISTORY_PUBKEY,
            stake_config_sysvar=SYSVAR_STAKE_CONFIG_ID,
            system_program_id=SYS_PROGRAM_ID,
            stake_program_id=STAKE_PROGRAM_ID,
        )
    )


def remove_validator_from_pool(params: RemoveValidatorFromPoolParams) -> TransactionInstruction:
    """Creates instruction to remove a validator from the pool."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.staker, is_signer=True, is_writable=False),
            AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.new_stake_authority, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.transient_stake, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.destination_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.REMOVE_VALIDATOR_FROM_POOL,
                args=None
            )
        )
    )


def remove_validator_from_pool_with_vote(
    program_id: PublicKey,
    stake_pool: PublicKey,
    staker: PublicKey,
    validator_list: PublicKey,
    new_stake_authority: PublicKey,
    validator: PublicKey,
    transient_stake_seed: int,
    destination_stake: PublicKey,
) -> TransactionInstruction:
    """Creates instruction to remove a validator based on their vote account address."""
    (withdraw_authority, seed) = find_withdraw_authority_program_address(program_id, stake_pool)
    (validator_stake, seed) = find_stake_program_address(program_id, validator, stake_pool)
    (transient_stake, seed) = find_transient_stake_program_address(
        program_id, validator, stake_pool, transient_stake_seed)
    return remove_validator_from_pool(
        RemoveValidatorFromPoolParams(
            program_id=STAKE_POOL_PROGRAM_ID,
            stake_pool=stake_pool,
            staker=staker,
            withdraw_authority=withdraw_authority,
            new_stake_authority=new_stake_authority,
            validator_list=validator_list,
            validator_stake=validator_stake,
            transient_stake=transient_stake,
            destination_stake=destination_stake,
            clock_sysvar=SYSVAR_CLOCK_PUBKEY,
            stake_program_id=STAKE_PROGRAM_ID,
        )
    )


def deposit_stake(params: DepositStakeParams) -> TransactionInstruction:
    """Creates a transaction instruction to deposit SOL into a stake pool."""
    keys = [
        AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.deposit_authority, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.deposit_stake, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.validator_stake, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.destination_pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.referral_pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.stake_history_sysvar, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.token_program_id, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
    ]
    return TransactionInstruction(
        keys=keys,
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.DEPOSIT_STAKE,
                args=None,
            )
        )
    )


def withdraw_stake(params: WithdrawStakeParams) -> TransactionInstruction:
    """Creates a transaction instruction to withdraw SOL from a stake pool."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.validator_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.destination_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.destination_stake_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.source_transfer_authority, is_signer=True, is_writable=False),
            AccountMeta(pubkey=params.source_pool_account, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.token_program_id, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.WITHDRAW_STAKE,
                args={'amount': params.amount}
            )
        )
    )


def deposit_sol(params: DepositSolParams) -> TransactionInstruction:
    """Creates a transaction instruction to deposit SOL into a stake pool."""
    keys = [
        AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.funding_account, is_signer=True, is_writable=True),
        AccountMeta(pubkey=params.destination_pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.referral_pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.system_program_id, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.token_program_id, is_signer=False, is_writable=False),
    ]
    if params.deposit_authority:
        keys.append(AccountMeta(pubkey=params.deposit_authority, is_signer=True, is_writable=False))
    return TransactionInstruction(
        keys=keys,
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.DEPOSIT_SOL,
                args={'amount': params.amount}
            )
        )
    )


def withdraw_sol(params: WithdrawSolParams) -> TransactionInstruction:
    """Creates a transaction instruction to withdraw SOL from a stake pool."""
    keys = [
        AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.source_transfer_authority, is_signer=True, is_writable=False),
        AccountMeta(pubkey=params.source_pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.destination_system_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.stake_history_sysvar, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.token_program_id, is_signer=False, is_writable=False),
    ]

    if params.sol_withdraw_authority:
        AccountMeta(pubkey=params.sol_withdraw_authority, is_signer=True, is_writable=False)

    return TransactionInstruction(
        keys=keys,
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.WITHDRAW_SOL,
                args={'amount': params.amount}
            )
        )
    )


def update_validator_list_balance(params: UpdateValidatorListBalanceParams) -> TransactionInstruction:
    """Creates instruction to update a set of validators in the stake pool."""
    keys = [
        AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=True),
        AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.stake_history_sysvar, is_signer=False, is_writable=False),
        AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
    ]
    keys.extend([
        AccountMeta(pubkey=pubkey, is_signer=False, is_writable=True)
        for pubkey in params.validator_and_transient_stake_pairs
    ])
    return TransactionInstruction(
        keys=keys,
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.UPDATE_VALIDATOR_LIST_BALANCE,
                args={'start_index': params.start_index, 'no_merge': params.no_merge}
            )
        )
    )


def update_stake_pool_balance(params: UpdateStakePoolBalanceParams) -> TransactionInstruction:
    """Creates instruction to update the overall stake pool balance."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.manager_fee_account, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.pool_mint, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.token_program_id, is_signer=False, is_writable=False),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.UPDATE_STAKE_POOL_BALANCE,
                args=None,
            )
        )
    )


def cleanup_removed_validator_entries(params: CleanupRemovedValidatorEntriesParams) -> TransactionInstruction:
    """Creates instruction to cleanup removed validator entries."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.CLEANUP_REMOVED_VALIDATOR_ENTRIES,
                args=None,
            )
        )
    )


def increase_validator_stake(params: IncreaseValidatorStakeParams) -> TransactionInstruction:
    """Creates instruction to increase the stake on a validator."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.staker, is_signer=True, is_writable=False),
            AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.reserve_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.transient_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_vote, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.rent_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_history_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_config_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.system_program_id, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.INCREASE_VALIDATOR_STAKE,
                args={
                    'lamports': params.lamports,
                    'transient_stake_seed': params.transient_stake_seed
                }
            )
        )
    )


def decrease_validator_stake(params: DecreaseValidatorStakeParams) -> TransactionInstruction:
    """Creates instruction to decrease the stake on a validator."""
    return TransactionInstruction(
        keys=[
            AccountMeta(pubkey=params.stake_pool, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.staker, is_signer=True, is_writable=False),
            AccountMeta(pubkey=params.withdraw_authority, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.validator_list, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.validator_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.transient_stake, is_signer=False, is_writable=True),
            AccountMeta(pubkey=params.clock_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.rent_sysvar, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.system_program_id, is_signer=False, is_writable=False),
            AccountMeta(pubkey=params.stake_program_id, is_signer=False, is_writable=False),
        ],
        program_id=params.program_id,
        data=INSTRUCTIONS_LAYOUT.build(
            dict(
                instruction_type=InstructionType.DECREASE_VALIDATOR_STAKE,
                args={
                    'lamports': params.lamports,
                    'transient_stake_seed': params.transient_stake_seed
                }
            )
        )
    )
