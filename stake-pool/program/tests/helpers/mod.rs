#![allow(dead_code)]

use {
    borsh::BorshDeserialize,
    solana_program::{
        borsh1::{get_instance_packed_len, get_packed_len, try_from_slice_unchecked},
        hash::Hash,
        instruction::Instruction,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        stake, system_instruction, system_program,
    },
    solana_program_test::{processor, BanksClient, ProgramTest, ProgramTestContext},
    solana_sdk::{
        account::{Account as SolanaAccount, WritableAccount},
        clock::{Clock, Epoch},
        compute_budget::ComputeBudgetInstruction,
        signature::{Keypair, Signer},
        transaction::Transaction,
        transport::TransportError,
    },
    solana_vote_program::{
        self, vote_instruction,
        vote_state::{VoteInit, VoteState, VoteStateVersions},
    },
    spl_stake_pool::{
        find_deposit_authority_program_address, find_ephemeral_stake_program_address,
        find_stake_program_address, find_transient_stake_program_address,
        find_withdraw_authority_program_address, id,
        inline_mpl_token_metadata::{self, pda::find_metadata_account},
        instruction, minimum_delegation,
        processor::Processor,
        state::{self, FeeType, FutureEpoch, StakePool, ValidatorList},
        MAX_VALIDATORS_TO_UPDATE, MINIMUM_RESERVE_LAMPORTS,
    },
    spl_token_2022::{
        extension::{ExtensionType, StateWithExtensionsOwned},
        native_mint,
        state::{Account, Mint},
    },
    std::{convert::TryInto, num::NonZeroU32},
};

pub const FIRST_NORMAL_EPOCH: u64 = 15;
pub const TEST_STAKE_AMOUNT: u64 = 1_500_000_000;
pub const MAX_TEST_VALIDATORS: u32 = 10_000;
pub const DEFAULT_VALIDATOR_STAKE_SEED: Option<NonZeroU32> = NonZeroU32::new(1_010);
pub const DEFAULT_TRANSIENT_STAKE_SEED: u64 = 42;
pub const STAKE_ACCOUNT_RENT_EXEMPTION: u64 = 2_282_880;
const ACCOUNT_RENT_EXEMPTION: u64 = 1_000_000_000; // go with something big to be safe

pub fn program_test() -> ProgramTest {
    let mut program_test = ProgramTest::new("spl_stake_pool", id(), processor!(Processor::process));
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );
    program_test
}

pub fn program_test_with_metadata_program() -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.add_program("spl_stake_pool", id(), processor!(Processor::process));
    program_test.add_program("mpl_token_metadata", inline_mpl_token_metadata::id(), None);
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );
    program_test
}

pub async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> SolanaAccount {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("client error")
        .expect("account not found")
}

#[allow(clippy::too_many_arguments)]
pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    pool_mint: &Keypair,
    manager: &Pubkey,
    decimals: u8,
    extension_types: &[ExtensionType],
) -> Result<(), TransportError> {
    assert!(extension_types.is_empty() || program_id != &spl_token::id());
    let rent = banks_client.get_rent().await.unwrap();
    let space = ExtensionType::try_calculate_account_len::<Mint>(extension_types).unwrap();
    let mint_rent = rent.minimum_balance(space);
    let mint_pubkey = pool_mint.pubkey();

    let mut instructions = vec![system_instruction::create_account(
        &payer.pubkey(),
        &mint_pubkey,
        mint_rent,
        space as u64,
        program_id,
    )];
    for extension_type in extension_types {
        let instruction = match extension_type {
            ExtensionType::MintCloseAuthority =>
                spl_token_2022::instruction::initialize_mint_close_authority(
                    program_id,
                    &mint_pubkey,
                    Some(manager),
                ),
            ExtensionType::DefaultAccountState =>
                spl_token_2022::extension::default_account_state::instruction::initialize_default_account_state(
                    program_id,
                    &mint_pubkey,
                    &spl_token_2022::state::AccountState::Initialized,
                ),
            ExtensionType::TransferFeeConfig => spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                program_id,
                &mint_pubkey,
                Some(manager),
                Some(manager),
                100,
                1_000_000,
            ),
            ExtensionType::InterestBearingConfig => spl_token_2022::extension::interest_bearing_mint::instruction::initialize(
                program_id,
                &mint_pubkey,
                Some(*manager),
                600,
            ),
            ExtensionType::NonTransferable =>
                spl_token_2022::instruction::initialize_non_transferable_mint(program_id, &mint_pubkey),
            _ => unimplemented!(),
        };
        instructions.push(instruction.unwrap());
    }
    instructions.push(
        spl_token_2022::instruction::initialize_mint(
            program_id,
            &pool_mint.pubkey(),
            manager,
            None,
            decimals,
        )
        .unwrap(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer, pool_mint],
        *recent_blockhash,
    );
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn transfer(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    recipient: &Pubkey,
    amount: u64,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            recipient,
            amount,
        )],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[allow(clippy::too_many_arguments)]
pub async fn transfer_spl_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
    decimals: u8,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token_2022::instruction::transfer_checked(
            program_id,
            source,
            mint,
            destination,
            &authority.pubkey(),
            &[],
            amount,
            decimals,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[allow(clippy::too_many_arguments)]
pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    account: &Keypair,
    pool_mint: &Pubkey,
    authority: &Keypair,
    extensions: &[ExtensionType],
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let space = ExtensionType::try_calculate_account_len::<Account>(extensions).unwrap();
    let account_rent = rent.minimum_balance(space);

    let mut instructions = vec![system_instruction::create_account(
        &payer.pubkey(),
        &account.pubkey(),
        account_rent,
        space as u64,
        program_id,
    )];

    for extension in extensions {
        match extension {
            ExtensionType::ImmutableOwner => instructions.push(
                spl_token_2022::instruction::initialize_immutable_owner(
                    program_id,
                    &account.pubkey(),
                )
                .unwrap(),
            ),
            ExtensionType::TransferFeeAmount
            | ExtensionType::MemoTransfer
            | ExtensionType::CpiGuard
            | ExtensionType::NonTransferableAccount => (),
            _ => unimplemented!(),
        };
    }

    instructions.push(
        spl_token_2022::instruction::initialize_account(
            program_id,
            &account.pubkey(),
            pool_mint,
            &authority.pubkey(),
        )
        .unwrap(),
    );

    let mut signers = vec![payer, account];
    for extension in extensions {
        match extension {
            ExtensionType::MemoTransfer => {
                signers.push(authority);
                instructions.push(
                spl_token_2022::extension::memo_transfer::instruction::enable_required_transfer_memos(
                    program_id,
                    &account.pubkey(),
                    &authority.pubkey(),
                    &[],
                )
                .unwrap()
                )
            }
            ExtensionType::CpiGuard => {
                signers.push(authority);
                instructions.push(
                    spl_token_2022::extension::cpi_guard::instruction::enable_cpi_guard(
                        program_id,
                        &account.pubkey(),
                        &authority.pubkey(),
                        &[],
                    )
                    .unwrap(),
                )
            }
            ExtensionType::ImmutableOwner
            | ExtensionType::TransferFeeAmount
            | ExtensionType::NonTransferableAccount => (),
            _ => unimplemented!(),
        }
    }

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        *recent_blockhash,
    );
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn close_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    account: &Pubkey,
    lamports_destination: &Pubkey,
    manager: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token_2022::instruction::close_account(
            program_id,
            account,
            lamports_destination,
            &manager.pubkey(),
            &[],
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, manager], *recent_blockhash);
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn freeze_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    account: &Pubkey,
    pool_mint: &Pubkey,
    manager: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token_2022::instruction::freeze_account(
            program_id,
            account,
            pool_mint,
            &manager.pubkey(),
            &[],
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, manager], *recent_blockhash);
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

#[allow(clippy::too_many_arguments)]
pub async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token_2022::instruction::mint_to(
            program_id,
            mint,
            account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        *recent_blockhash,
    );
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

#[allow(clippy::too_many_arguments)]
pub async fn burn_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    mint: &Pubkey,
    account: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token_2022::instruction::burn(
            program_id,
            account,
            mint,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        *recent_blockhash,
    );
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info = StateWithExtensionsOwned::<Account>::unpack(token_account.data).unwrap();
    account_info.base.amount
}

#[derive(Clone, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub key: u8,
    pub update_authority: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<u8>>,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
}

pub async fn get_metadata_account(banks_client: &mut BanksClient, token_mint: &Pubkey) -> Metadata {
    let (token_metadata, _) = find_metadata_account(token_mint);
    let token_metadata_account = banks_client
        .get_account(token_metadata)
        .await
        .unwrap()
        .unwrap();
    try_from_slice_unchecked(token_metadata_account.data.as_slice()).unwrap()
}

pub async fn get_token_supply(banks_client: &mut BanksClient, mint: &Pubkey) -> u64 {
    let mint_account = banks_client.get_account(*mint).await.unwrap().unwrap();
    let account_info = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data).unwrap();
    account_info.base.supply
}

#[allow(clippy::too_many_arguments)]
pub async fn delegate_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    account: &Pubkey,
    manager: &Keypair,
    delegate: &Pubkey,
    amount: u64,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token_2022::instruction::approve(
            program_id,
            account,
            delegate,
            &manager.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, manager],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn revoke_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    account: &Pubkey,
    manager: &Keypair,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[
            spl_token_2022::instruction::revoke(program_id, account, &manager.pubkey(), &[])
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, manager],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[allow(clippy::too_many_arguments)]
pub async fn create_stake_pool(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool: &Keypair,
    validator_list: &Keypair,
    reserve_stake: &Pubkey,
    token_program_id: &Pubkey,
    pool_mint: &Pubkey,
    pool_token_account: &Pubkey,
    manager: &Keypair,
    staker: &Pubkey,
    withdraw_authority: &Pubkey,
    stake_deposit_authority: &Option<Keypair>,
    epoch_fee: &state::Fee,
    withdrawal_fee: &state::Fee,
    deposit_fee: &state::Fee,
    referral_fee: u8,
    sol_deposit_fee: &state::Fee,
    sol_referral_fee: u8,
    max_validators: u32,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(get_packed_len::<state::StakePool>());
    let validator_list_size =
        get_instance_packed_len(&state::ValidatorList::new(max_validators)).unwrap();
    let rent_validator_list = rent.minimum_balance(validator_list_size);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool.pubkey(),
                rent_stake_pool,
                get_packed_len::<state::StakePool>() as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &validator_list.pubkey(),
                rent_validator_list,
                validator_list_size as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool.pubkey(),
                &manager.pubkey(),
                staker,
                withdraw_authority,
                &validator_list.pubkey(),
                reserve_stake,
                pool_mint,
                pool_token_account,
                token_program_id,
                stake_deposit_authority.as_ref().map(|k| k.pubkey()),
                *epoch_fee,
                *withdrawal_fee,
                *deposit_fee,
                referral_fee,
                max_validators,
            ),
            instruction::set_fee(
                &id(),
                &stake_pool.pubkey(),
                &manager.pubkey(),
                FeeType::SolDeposit(*sol_deposit_fee),
            ),
            instruction::set_fee(
                &id(),
                &stake_pool.pubkey(),
                &manager.pubkey(),
                FeeType::SolReferral(sol_referral_fee),
            ),
        ],
        Some(&payer.pubkey()),
    );
    let mut signers = vec![payer, stake_pool, validator_list, manager];
    if let Some(stake_deposit_authority) = stake_deposit_authority.as_ref() {
        signers.push(stake_deposit_authority);
    }
    transaction.sign(&signers, *recent_blockhash);
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn create_vote(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    validator: &Keypair,
    vote: &Keypair,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_voter = rent.minimum_balance(VoteState::size_of());

    let mut instructions = vec![system_instruction::create_account(
        &payer.pubkey(),
        &validator.pubkey(),
        rent.minimum_balance(0),
        0,
        &system_program::id(),
    )];
    instructions.append(&mut vote_instruction::create_account_with_config(
        &payer.pubkey(),
        &vote.pubkey(),
        &VoteInit {
            node_pubkey: validator.pubkey(),
            authorized_voter: validator.pubkey(),
            ..VoteInit::default()
        },
        rent_voter,
        vote_instruction::CreateVoteAccountConfig {
            space: VoteState::size_of() as u64,
            ..Default::default()
        },
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[validator, vote, payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn create_independent_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
    authorized: &stake::state::Authorized,
    lockup: &stake::state::Lockup,
    stake_amount: u64,
) -> u64 {
    let rent = banks_client.get_rent().await.unwrap();
    let lamports =
        rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>()) + stake_amount;

    let transaction = Transaction::new_signed_with_payer(
        &stake::instruction::create_account(
            &payer.pubkey(),
            &stake.pubkey(),
            authorized,
            lockup,
            lamports,
        ),
        Some(&payer.pubkey()),
        &[payer, stake],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    lamports
}

pub async fn create_blank_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
) -> u64 {
    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake.pubkey(),
            lamports,
            std::mem::size_of::<stake::state::StakeStateV2>() as u64,
            &stake::program::id(),
        )],
        Some(&payer.pubkey()),
        &[payer, stake],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    lamports
}

pub async fn delegate_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Pubkey,
    authorized: &Keypair,
    vote: &Pubkey,
) {
    let mut transaction = Transaction::new_with_payer(
        &[stake::instruction::delegate_stake(
            stake,
            &authorized.pubkey(),
            vote,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authorized], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn stake_get_minimum_delegation(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> u64 {
    let transaction = Transaction::new_signed_with_payer(
        &[stake::instruction::get_minimum_delegation()],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    let mut data = banks_client
        .simulate_transaction(transaction)
        .await
        .unwrap()
        .simulation_details
        .unwrap()
        .return_data
        .unwrap()
        .data;
    data.resize(8, 0);
    data.try_into().map(u64::from_le_bytes).unwrap()
}

pub async fn stake_pool_get_minimum_delegation(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> u64 {
    let stake_minimum = stake_get_minimum_delegation(banks_client, payer, recent_blockhash).await;
    minimum_delegation(stake_minimum)
}

pub async fn authorize_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Pubkey,
    authorized: &Keypair,
    new_authorized: &Pubkey,
    stake_authorize: stake::state::StakeAuthorize,
) {
    let mut transaction = Transaction::new_with_payer(
        &[stake::instruction::authorize(
            stake,
            &authorized.pubkey(),
            new_authorized,
            stake_authorize,
            None,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authorized], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn create_unknown_validator_stake(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool: &Pubkey,
    lamports: u64,
) -> ValidatorStakeAccount {
    let mut unknown_stake = ValidatorStakeAccount::new(stake_pool, NonZeroU32::new(1), 222);
    create_vote(
        banks_client,
        payer,
        recent_blockhash,
        &unknown_stake.validator,
        &unknown_stake.vote,
    )
    .await;
    let user = Keypair::new();
    let fake_validator_stake = Keypair::new();
    let stake_minimum_delegation =
        stake_get_minimum_delegation(banks_client, payer, recent_blockhash).await;
    let current_minimum_delegation = minimum_delegation(stake_minimum_delegation);
    create_independent_stake_account(
        banks_client,
        payer,
        recent_blockhash,
        &fake_validator_stake,
        &stake::state::Authorized {
            staker: user.pubkey(),
            withdrawer: user.pubkey(),
        },
        &stake::state::Lockup::default(),
        current_minimum_delegation + lamports,
    )
    .await;
    delegate_stake_account(
        banks_client,
        payer,
        recent_blockhash,
        &fake_validator_stake.pubkey(),
        &user,
        &unknown_stake.vote.pubkey(),
    )
    .await;
    unknown_stake.stake_account = fake_validator_stake.pubkey();
    unknown_stake
}

pub struct ValidatorStakeAccount {
    pub stake_account: Pubkey,
    pub transient_stake_account: Pubkey,
    pub transient_stake_seed: u64,
    pub validator_stake_seed: Option<NonZeroU32>,
    pub vote: Keypair,
    pub validator: Keypair,
    pub stake_pool: Pubkey,
}

impl ValidatorStakeAccount {
    pub fn new(
        stake_pool: &Pubkey,
        validator_stake_seed: Option<NonZeroU32>,
        transient_stake_seed: u64,
    ) -> Self {
        let validator = Keypair::new();
        let vote = Keypair::new();
        let (stake_account, _) =
            find_stake_program_address(&id(), &vote.pubkey(), stake_pool, validator_stake_seed);
        let (transient_stake_account, _) = find_transient_stake_program_address(
            &id(),
            &vote.pubkey(),
            stake_pool,
            transient_stake_seed,
        );
        ValidatorStakeAccount {
            stake_account,
            transient_stake_account,
            transient_stake_seed,
            validator_stake_seed,
            vote,
            validator,
            stake_pool: *stake_pool,
        }
    }
}

pub struct StakePoolAccounts {
    pub stake_pool: Keypair,
    pub validator_list: Keypair,
    pub reserve_stake: Keypair,
    pub token_program_id: Pubkey,
    pub pool_mint: Keypair,
    pub pool_fee_account: Keypair,
    pub pool_decimals: u8,
    pub manager: Keypair,
    pub staker: Keypair,
    pub withdraw_authority: Pubkey,
    pub stake_deposit_authority: Pubkey,
    pub stake_deposit_authority_keypair: Option<Keypair>,
    pub epoch_fee: state::Fee,
    pub withdrawal_fee: state::Fee,
    pub deposit_fee: state::Fee,
    pub referral_fee: u8,
    pub sol_deposit_fee: state::Fee,
    pub sol_referral_fee: u8,
    pub max_validators: u32,
    pub compute_unit_limit: Option<u32>,
}

impl StakePoolAccounts {
    pub fn new_with_deposit_authority(stake_deposit_authority: Keypair) -> Self {
        Self {
            stake_deposit_authority: stake_deposit_authority.pubkey(),
            stake_deposit_authority_keypair: Some(stake_deposit_authority),
            ..Default::default()
        }
    }

    pub fn new_with_token_program(token_program_id: Pubkey) -> Self {
        Self {
            token_program_id,
            ..Default::default()
        }
    }

    pub fn calculate_fee(&self, amount: u64) -> u64 {
        (amount * self.epoch_fee.numerator + self.epoch_fee.denominator - 1)
            / self.epoch_fee.denominator
    }

    pub fn calculate_withdrawal_fee(&self, pool_tokens: u64) -> u64 {
        (pool_tokens * self.withdrawal_fee.numerator + self.withdrawal_fee.denominator - 1)
            / self.withdrawal_fee.denominator
    }

    pub fn calculate_inverse_withdrawal_fee(&self, pool_tokens: u64) -> u64 {
        (pool_tokens * self.withdrawal_fee.denominator + self.withdrawal_fee.denominator - 1)
            / (self.withdrawal_fee.denominator - self.withdrawal_fee.numerator)
    }

    pub fn calculate_referral_fee(&self, deposit_fee_collected: u64) -> u64 {
        deposit_fee_collected * self.referral_fee as u64 / 100
    }

    pub fn calculate_sol_deposit_fee(&self, pool_tokens: u64) -> u64 {
        (pool_tokens * self.sol_deposit_fee.numerator + self.sol_deposit_fee.denominator - 1)
            / self.sol_deposit_fee.denominator
    }

    pub fn calculate_sol_referral_fee(&self, deposit_fee_collected: u64) -> u64 {
        deposit_fee_collected * self.sol_referral_fee as u64 / 100
    }

    pub async fn initialize_stake_pool(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        reserve_lamports: u64,
    ) -> Result<(), TransportError> {
        create_mint(
            banks_client,
            payer,
            recent_blockhash,
            &self.token_program_id,
            &self.pool_mint,
            &self.withdraw_authority,
            self.pool_decimals,
            &[],
        )
        .await?;
        create_token_account(
            banks_client,
            payer,
            recent_blockhash,
            &self.token_program_id,
            &self.pool_fee_account,
            &self.pool_mint.pubkey(),
            &self.manager,
            &[],
        )
        .await?;
        create_independent_stake_account(
            banks_client,
            payer,
            recent_blockhash,
            &self.reserve_stake,
            &stake::state::Authorized {
                staker: self.withdraw_authority,
                withdrawer: self.withdraw_authority,
            },
            &stake::state::Lockup::default(),
            reserve_lamports,
        )
        .await;
        create_stake_pool(
            banks_client,
            payer,
            recent_blockhash,
            &self.stake_pool,
            &self.validator_list,
            &self.reserve_stake.pubkey(),
            &self.token_program_id,
            &self.pool_mint.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.manager,
            &self.staker.pubkey(),
            &self.withdraw_authority,
            &self.stake_deposit_authority_keypair,
            &self.epoch_fee,
            &self.withdrawal_fee,
            &self.deposit_fee,
            self.referral_fee,
            &self.sol_deposit_fee,
            self.sol_referral_fee,
            self.max_validators,
        )
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_stake_with_slippage(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake: &Pubkey,
        pool_account: &Pubkey,
        validator_stake_account: &Pubkey,
        current_staker: &Keypair,
        minimum_pool_tokens_out: u64,
    ) -> Option<TransportError> {
        let mut instructions = instruction::deposit_stake_with_slippage(
            &id(),
            &self.stake_pool.pubkey(),
            &self.validator_list.pubkey(),
            &self.withdraw_authority,
            stake,
            &current_staker.pubkey(),
            validator_stake_account,
            &self.reserve_stake.pubkey(),
            pool_account,
            &self.pool_fee_account.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.pool_mint.pubkey(),
            &self.token_program_id,
            minimum_pool_tokens_out,
        );
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, current_staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake: &Pubkey,
        pool_account: &Pubkey,
        validator_stake_account: &Pubkey,
        current_staker: &Keypair,
    ) -> Option<TransportError> {
        self.deposit_stake_with_referral(
            banks_client,
            payer,
            recent_blockhash,
            stake,
            pool_account,
            validator_stake_account,
            current_staker,
            &self.pool_fee_account.pubkey(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_stake_with_referral(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake: &Pubkey,
        pool_account: &Pubkey,
        validator_stake_account: &Pubkey,
        current_staker: &Keypair,
        referrer: &Pubkey,
    ) -> Option<TransportError> {
        let mut signers = vec![payer, current_staker];
        let mut instructions =
            if let Some(stake_deposit_authority) = self.stake_deposit_authority_keypair.as_ref() {
                signers.push(stake_deposit_authority);
                instruction::deposit_stake_with_authority(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.validator_list.pubkey(),
                    &self.stake_deposit_authority,
                    &self.withdraw_authority,
                    stake,
                    &current_staker.pubkey(),
                    validator_stake_account,
                    &self.reserve_stake.pubkey(),
                    pool_account,
                    &self.pool_fee_account.pubkey(),
                    referrer,
                    &self.pool_mint.pubkey(),
                    &self.token_program_id,
                )
            } else {
                instruction::deposit_stake(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.validator_list.pubkey(),
                    &self.withdraw_authority,
                    stake,
                    &current_staker.pubkey(),
                    validator_stake_account,
                    &self.reserve_stake.pubkey(),
                    pool_account,
                    &self.pool_fee_account.pubkey(),
                    referrer,
                    &self.pool_mint.pubkey(),
                    &self.token_program_id,
                )
            };
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &signers,
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_sol(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        pool_account: &Pubkey,
        amount: u64,
        sol_deposit_authority: Option<&Keypair>,
    ) -> Option<TransportError> {
        let mut signers = vec![payer];
        let instruction = if let Some(sol_deposit_authority) = sol_deposit_authority {
            signers.push(sol_deposit_authority);
            instruction::deposit_sol_with_authority(
                &id(),
                &self.stake_pool.pubkey(),
                &sol_deposit_authority.pubkey(),
                &self.withdraw_authority,
                &self.reserve_stake.pubkey(),
                &payer.pubkey(),
                pool_account,
                &self.pool_fee_account.pubkey(),
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &self.token_program_id,
                amount,
            )
        } else {
            instruction::deposit_sol(
                &id(),
                &self.stake_pool.pubkey(),
                &self.withdraw_authority,
                &self.reserve_stake.pubkey(),
                &payer.pubkey(),
                pool_account,
                &self.pool_fee_account.pubkey(),
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &self.token_program_id,
                amount,
            )
        };
        let mut instructions = vec![instruction];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &signers,
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_sol_with_slippage(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        pool_account: &Pubkey,
        lamports_in: u64,
        minimum_pool_tokens_out: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::deposit_sol_with_slippage(
            &id(),
            &self.stake_pool.pubkey(),
            &self.withdraw_authority,
            &self.reserve_stake.pubkey(),
            &payer.pubkey(),
            pool_account,
            &self.pool_fee_account.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.pool_mint.pubkey(),
            &self.token_program_id,
            lamports_in,
            minimum_pool_tokens_out,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_stake_with_slippage(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake_recipient: &Pubkey,
        user_transfer_authority: &Keypair,
        pool_account: &Pubkey,
        validator_stake_account: &Pubkey,
        recipient_new_authority: &Pubkey,
        pool_tokens_in: u64,
        minimum_lamports_out: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::withdraw_stake_with_slippage(
            &id(),
            &self.stake_pool.pubkey(),
            &self.validator_list.pubkey(),
            &self.withdraw_authority,
            validator_stake_account,
            stake_recipient,
            recipient_new_authority,
            &user_transfer_authority.pubkey(),
            pool_account,
            &self.pool_fee_account.pubkey(),
            &self.pool_mint.pubkey(),
            &self.token_program_id,
            pool_tokens_in,
            minimum_lamports_out,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, user_transfer_authority],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake_recipient: &Pubkey,
        user_transfer_authority: &Keypair,
        pool_account: &Pubkey,
        validator_stake_account: &Pubkey,
        recipient_new_authority: &Pubkey,
        amount: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::withdraw_stake(
            &id(),
            &self.stake_pool.pubkey(),
            &self.validator_list.pubkey(),
            &self.withdraw_authority,
            validator_stake_account,
            stake_recipient,
            recipient_new_authority,
            &user_transfer_authority.pubkey(),
            pool_account,
            &self.pool_fee_account.pubkey(),
            &self.pool_mint.pubkey(),
            &self.token_program_id,
            amount,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, user_transfer_authority],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_sol_with_slippage(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        user: &Keypair,
        pool_account: &Pubkey,
        amount_in: u64,
        minimum_lamports_out: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::withdraw_sol_with_slippage(
            &id(),
            &self.stake_pool.pubkey(),
            &self.withdraw_authority,
            &user.pubkey(),
            pool_account,
            &self.reserve_stake.pubkey(),
            &user.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.pool_mint.pubkey(),
            &self.token_program_id,
            amount_in,
            minimum_lamports_out,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, user],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_sol(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        user: &Keypair,
        pool_account: &Pubkey,
        amount: u64,
        sol_withdraw_authority: Option<&Keypair>,
    ) -> Option<TransportError> {
        let mut signers = vec![payer, user];
        let instruction = if let Some(sol_withdraw_authority) = sol_withdraw_authority {
            signers.push(sol_withdraw_authority);
            instruction::withdraw_sol_with_authority(
                &id(),
                &self.stake_pool.pubkey(),
                &sol_withdraw_authority.pubkey(),
                &self.withdraw_authority,
                &user.pubkey(),
                pool_account,
                &self.reserve_stake.pubkey(),
                &user.pubkey(),
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &self.token_program_id,
                amount,
            )
        } else {
            instruction::withdraw_sol(
                &id(),
                &self.stake_pool.pubkey(),
                &self.withdraw_authority,
                &user.pubkey(),
                pool_account,
                &self.reserve_stake.pubkey(),
                &user.pubkey(),
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &self.token_program_id,
                amount,
            )
        };
        let mut instructions = vec![instruction];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &signers,
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub async fn get_stake_pool(&self, banks_client: &mut BanksClient) -> StakePool {
        let stake_pool_account = get_account(banks_client, &self.stake_pool.pubkey()).await;
        try_from_slice_unchecked::<StakePool>(stake_pool_account.data.as_slice()).unwrap()
    }

    pub async fn get_validator_list(&self, banks_client: &mut BanksClient) -> ValidatorList {
        let validator_list_account = get_account(banks_client, &self.validator_list.pubkey()).await;
        try_from_slice_unchecked::<ValidatorList>(validator_list_account.data.as_slice()).unwrap()
    }

    pub async fn update_validator_list_balance(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        len: usize,
        no_merge: bool,
    ) -> Option<TransportError> {
        let validator_list = self.get_validator_list(banks_client).await;
        let mut instructions = vec![instruction::update_validator_list_balance_chunk(
            &id(),
            &self.stake_pool.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            &self.reserve_stake.pubkey(),
            &validator_list,
            len,
            0,
            no_merge,
        )
        .unwrap()];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub async fn update_stake_pool_balance(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::update_stake_pool_balance(
            &id(),
            &self.stake_pool.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            &self.reserve_stake.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.pool_mint.pubkey(),
            &self.token_program_id,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub async fn cleanup_removed_validator_entries(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::cleanup_removed_validator_entries(
            &id(),
            &self.stake_pool.pubkey(),
            &self.validator_list.pubkey(),
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub async fn update_all(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        no_merge: bool,
    ) -> Option<TransportError> {
        let validator_list = self.get_validator_list(banks_client).await;
        let mut instructions = vec![];
        for (i, chunk) in validator_list
            .validators
            .chunks(MAX_VALIDATORS_TO_UPDATE)
            .enumerate()
        {
            instructions.push(
                instruction::update_validator_list_balance_chunk(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.withdraw_authority,
                    &self.validator_list.pubkey(),
                    &self.reserve_stake.pubkey(),
                    &validator_list,
                    chunk.len(),
                    i * MAX_VALIDATORS_TO_UPDATE,
                    no_merge,
                )
                .unwrap(),
            );
        }
        instructions.extend([
            instruction::update_stake_pool_balance(
                &id(),
                &self.stake_pool.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                &self.reserve_stake.pubkey(),
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &self.token_program_id,
            ),
            instruction::cleanup_removed_validator_entries(
                &id(),
                &self.stake_pool.pubkey(),
                &self.validator_list.pubkey(),
            ),
        ]);
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub async fn add_validator_to_pool(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake: &Pubkey,
        validator: &Pubkey,
        seed: Option<NonZeroU32>,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::add_validator_to_pool(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.reserve_stake.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            stake,
            validator,
            seed,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn remove_validator_from_pool(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_stake: &Pubkey,
        transient_stake: &Pubkey,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::remove_validator_from_pool(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            validator_stake,
            transient_stake,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn decrease_validator_stake_deprecated(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_stake: &Pubkey,
        transient_stake: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
    ) -> Option<TransportError> {
        #[allow(deprecated)]
        let mut instructions = vec![
            system_instruction::transfer(
                &payer.pubkey(),
                transient_stake,
                STAKE_ACCOUNT_RENT_EXEMPTION,
            ),
            instruction::decrease_validator_stake(
                &id(),
                &self.stake_pool.pubkey(),
                &self.staker.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                validator_stake,
                transient_stake,
                lamports,
                transient_stake_seed,
            ),
        ];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn decrease_validator_stake_with_reserve(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_stake: &Pubkey,
        transient_stake: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::decrease_validator_stake_with_reserve(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            &self.reserve_stake.pubkey(),
            validator_stake,
            transient_stake,
            lamports,
            transient_stake_seed,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn decrease_additional_validator_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_stake: &Pubkey,
        ephemeral_stake: &Pubkey,
        transient_stake: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
        ephemeral_stake_seed: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::decrease_additional_validator_stake(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            &self.reserve_stake.pubkey(),
            validator_stake,
            ephemeral_stake,
            transient_stake,
            lamports,
            transient_stake_seed,
            ephemeral_stake_seed,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn decrease_validator_stake_either(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_stake: &Pubkey,
        transient_stake: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
        instruction_type: DecreaseInstruction,
    ) -> Option<TransportError> {
        match instruction_type {
            DecreaseInstruction::Additional => {
                let ephemeral_stake_seed = 0;
                let ephemeral_stake = find_ephemeral_stake_program_address(
                    &id(),
                    &self.stake_pool.pubkey(),
                    ephemeral_stake_seed,
                )
                .0;
                self.decrease_additional_validator_stake(
                    banks_client,
                    payer,
                    recent_blockhash,
                    validator_stake,
                    &ephemeral_stake,
                    transient_stake,
                    lamports,
                    transient_stake_seed,
                    ephemeral_stake_seed,
                )
                .await
            }
            DecreaseInstruction::Reserve => {
                self.decrease_validator_stake_with_reserve(
                    banks_client,
                    payer,
                    recent_blockhash,
                    validator_stake,
                    transient_stake,
                    lamports,
                    transient_stake_seed,
                )
                .await
            }
            DecreaseInstruction::Deprecated =>
            {
                #[allow(deprecated)]
                self.decrease_validator_stake_deprecated(
                    banks_client,
                    payer,
                    recent_blockhash,
                    validator_stake,
                    transient_stake,
                    lamports,
                    transient_stake_seed,
                )
                .await
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn increase_validator_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        transient_stake: &Pubkey,
        validator_stake: &Pubkey,
        validator: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::increase_validator_stake(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            &self.reserve_stake.pubkey(),
            transient_stake,
            validator_stake,
            validator,
            lamports,
            transient_stake_seed,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn increase_additional_validator_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        ephemeral_stake: &Pubkey,
        transient_stake: &Pubkey,
        validator_stake: &Pubkey,
        validator: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
        ephemeral_stake_seed: u64,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::increase_additional_validator_stake(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.withdraw_authority,
            &self.validator_list.pubkey(),
            &self.reserve_stake.pubkey(),
            ephemeral_stake,
            transient_stake,
            validator_stake,
            validator,
            lamports,
            transient_stake_seed,
            ephemeral_stake_seed,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn increase_validator_stake_either(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        transient_stake: &Pubkey,
        validator_stake: &Pubkey,
        validator: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
        use_additional_instruction: bool,
    ) -> Option<TransportError> {
        if use_additional_instruction {
            let ephemeral_stake_seed = 0;
            let ephemeral_stake = find_ephemeral_stake_program_address(
                &id(),
                &self.stake_pool.pubkey(),
                ephemeral_stake_seed,
            )
            .0;
            self.increase_additional_validator_stake(
                banks_client,
                payer,
                recent_blockhash,
                &ephemeral_stake,
                transient_stake,
                validator_stake,
                validator,
                lamports,
                transient_stake_seed,
                ephemeral_stake_seed,
            )
            .await
        } else {
            self.increase_validator_stake(
                banks_client,
                payer,
                recent_blockhash,
                transient_stake,
                validator_stake,
                validator,
                lamports,
                transient_stake_seed,
            )
            .await
        }
    }

    pub async fn set_preferred_validator(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_type: instruction::PreferredValidatorType,
        validator: Option<Pubkey>,
    ) -> Option<TransportError> {
        let mut instructions = vec![instruction::set_preferred_validator(
            &id(),
            &self.stake_pool.pubkey(),
            &self.staker.pubkey(),
            &self.validator_list.pubkey(),
            validator_type,
            validator,
        )];
        self.maybe_add_compute_budget_instruction(&mut instructions);
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub fn state(&self) -> (state::StakePool, state::ValidatorList) {
        let (_, stake_withdraw_bump_seed) =
            find_withdraw_authority_program_address(&id(), &self.stake_pool.pubkey());
        let stake_pool = state::StakePool {
            account_type: state::AccountType::StakePool,
            manager: self.manager.pubkey(),
            staker: self.staker.pubkey(),
            stake_deposit_authority: self.stake_deposit_authority,
            stake_withdraw_bump_seed,
            validator_list: self.validator_list.pubkey(),
            reserve_stake: self.reserve_stake.pubkey(),
            pool_mint: self.pool_mint.pubkey(),
            manager_fee_account: self.pool_fee_account.pubkey(),
            token_program_id: self.token_program_id,
            total_lamports: 0,
            pool_token_supply: 0,
            last_update_epoch: 0,
            lockup: stake::state::Lockup::default(),
            epoch_fee: self.epoch_fee,
            next_epoch_fee: FutureEpoch::None,
            preferred_deposit_validator_vote_address: None,
            preferred_withdraw_validator_vote_address: None,
            stake_deposit_fee: state::Fee::default(),
            sol_deposit_fee: state::Fee::default(),
            stake_withdrawal_fee: state::Fee::default(),
            next_stake_withdrawal_fee: FutureEpoch::None,
            stake_referral_fee: 0,
            sol_referral_fee: 0,
            sol_deposit_authority: None,
            sol_withdraw_authority: None,
            sol_withdrawal_fee: state::Fee::default(),
            next_sol_withdrawal_fee: FutureEpoch::None,
            last_epoch_pool_token_supply: 0,
            last_epoch_total_lamports: 0,
        };
        let mut validator_list = ValidatorList::new(self.max_validators);
        validator_list.validators = vec![];
        (stake_pool, validator_list)
    }

    pub fn maybe_add_compute_budget_instruction(&self, instructions: &mut Vec<Instruction>) {
        if let Some(compute_unit_limit) = self.compute_unit_limit {
            instructions.insert(
                0,
                ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit),
            );
        }
    }
}
impl Default for StakePoolAccounts {
    fn default() -> Self {
        let stake_pool = Keypair::new();
        let validator_list = Keypair::new();
        let stake_pool_address = &stake_pool.pubkey();
        let (stake_deposit_authority, _) =
            find_deposit_authority_program_address(&id(), stake_pool_address);
        let (withdraw_authority, _) =
            find_withdraw_authority_program_address(&id(), stake_pool_address);
        let reserve_stake = Keypair::new();
        let pool_mint = Keypair::new();
        let pool_fee_account = Keypair::new();
        let manager = Keypair::new();
        let staker = Keypair::new();

        Self {
            stake_pool,
            validator_list,
            reserve_stake,
            token_program_id: spl_token::id(),
            pool_mint,
            pool_fee_account,
            pool_decimals: native_mint::DECIMALS,
            manager,
            staker,
            withdraw_authority,
            stake_deposit_authority,
            stake_deposit_authority_keypair: None,
            epoch_fee: state::Fee {
                numerator: 1,
                denominator: 100,
            },
            withdrawal_fee: state::Fee {
                numerator: 3,
                denominator: 1000,
            },
            deposit_fee: state::Fee {
                numerator: 1,
                denominator: 1000,
            },
            referral_fee: 25,
            sol_deposit_fee: state::Fee {
                numerator: 3,
                denominator: 100,
            },
            sol_referral_fee: 50,
            max_validators: MAX_TEST_VALIDATORS,
            compute_unit_limit: None,
        }
    }
}

pub async fn simple_add_validator_to_pool(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool_accounts: &StakePoolAccounts,
    sol_deposit_authority: Option<&Keypair>,
) -> ValidatorStakeAccount {
    let validator_stake = ValidatorStakeAccount::new(
        &stake_pool_accounts.stake_pool.pubkey(),
        DEFAULT_VALIDATOR_STAKE_SEED,
        DEFAULT_TRANSIENT_STAKE_SEED,
    );

    let rent = banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    let current_minimum_delegation =
        stake_pool_get_minimum_delegation(banks_client, payer, recent_blockhash).await;

    let pool_token_account = Keypair::new();
    create_token_account(
        banks_client,
        payer,
        recent_blockhash,
        &stake_pool_accounts.token_program_id,
        &pool_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        payer,
        &[],
    )
    .await
    .unwrap();
    let error = stake_pool_accounts
        .deposit_sol(
            banks_client,
            payer,
            recent_blockhash,
            &pool_token_account.pubkey(),
            stake_rent + current_minimum_delegation,
            sol_deposit_authority,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    create_vote(
        banks_client,
        payer,
        recent_blockhash,
        &validator_stake.validator,
        &validator_stake.vote,
    )
    .await;

    let error = stake_pool_accounts
        .add_validator_to_pool(
            banks_client,
            payer,
            recent_blockhash,
            &validator_stake.stake_account,
            &validator_stake.vote.pubkey(),
            validator_stake.validator_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    validator_stake
}

#[derive(Debug)]
pub struct DepositStakeAccount {
    pub authority: Keypair,
    pub stake: Keypair,
    pub pool_account: Keypair,
    pub stake_lamports: u64,
    pub pool_tokens: u64,
    pub vote_account: Pubkey,
    pub validator_stake_account: Pubkey,
}

impl DepositStakeAccount {
    pub fn new_with_vote(
        vote_account: Pubkey,
        validator_stake_account: Pubkey,
        stake_lamports: u64,
    ) -> Self {
        let authority = Keypair::new();
        let stake = Keypair::new();
        let pool_account = Keypair::new();
        Self {
            authority,
            stake,
            pool_account,
            vote_account,
            validator_stake_account,
            stake_lamports,
            pool_tokens: 0,
        }
    }

    pub async fn create_and_delegate(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        let lockup = stake::state::Lockup::default();
        let authorized = stake::state::Authorized {
            staker: self.authority.pubkey(),
            withdrawer: self.authority.pubkey(),
        };
        create_independent_stake_account(
            banks_client,
            payer,
            recent_blockhash,
            &self.stake,
            &authorized,
            &lockup,
            self.stake_lamports,
        )
        .await;
        delegate_stake_account(
            banks_client,
            payer,
            recent_blockhash,
            &self.stake.pubkey(),
            &self.authority,
            &self.vote_account,
        )
        .await;
    }

    pub async fn deposit_stake(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake_pool_accounts: &StakePoolAccounts,
    ) {
        // make pool token account
        create_token_account(
            banks_client,
            payer,
            recent_blockhash,
            &stake_pool_accounts.token_program_id,
            &self.pool_account,
            &stake_pool_accounts.pool_mint.pubkey(),
            &self.authority,
            &[],
        )
        .await
        .unwrap();

        let error = stake_pool_accounts
            .deposit_stake(
                banks_client,
                payer,
                recent_blockhash,
                &self.stake.pubkey(),
                &self.pool_account.pubkey(),
                &self.validator_stake_account,
                &self.authority,
            )
            .await;
        self.pool_tokens = get_token_balance(banks_client, &self.pool_account.pubkey()).await;
        assert!(error.is_none(), "{:?}", error);
    }
}

pub async fn simple_deposit_stake(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool_accounts: &StakePoolAccounts,
    validator_stake_account: &ValidatorStakeAccount,
    stake_lamports: u64,
) -> Option<DepositStakeAccount> {
    let authority = Keypair::new();
    // make stake account
    let stake = Keypair::new();
    let lockup = stake::state::Lockup::default();
    let authorized = stake::state::Authorized {
        staker: authority.pubkey(),
        withdrawer: authority.pubkey(),
    };
    create_independent_stake_account(
        banks_client,
        payer,
        recent_blockhash,
        &stake,
        &authorized,
        &lockup,
        stake_lamports,
    )
    .await;
    let vote_account = validator_stake_account.vote.pubkey();
    delegate_stake_account(
        banks_client,
        payer,
        recent_blockhash,
        &stake.pubkey(),
        &authority,
        &vote_account,
    )
    .await;
    // make pool token account
    let pool_account = Keypair::new();
    create_token_account(
        banks_client,
        payer,
        recent_blockhash,
        &stake_pool_accounts.token_program_id,
        &pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &authority,
        &[],
    )
    .await
    .unwrap();

    let validator_stake_account = validator_stake_account.stake_account;
    let error = stake_pool_accounts
        .deposit_stake(
            banks_client,
            payer,
            recent_blockhash,
            &stake.pubkey(),
            &pool_account.pubkey(),
            &validator_stake_account,
            &authority,
        )
        .await;
    // backwards, but oh well!
    if error.is_some() {
        return None;
    }

    let pool_tokens = get_token_balance(banks_client, &pool_account.pubkey()).await;

    Some(DepositStakeAccount {
        authority,
        stake,
        pool_account,
        stake_lamports,
        pool_tokens,
        vote_account,
        validator_stake_account,
    })
}

pub async fn get_validator_list_sum(
    banks_client: &mut BanksClient,
    reserve_stake: &Pubkey,
    validator_list: &Pubkey,
) -> u64 {
    let validator_list = banks_client
        .get_account(*validator_list)
        .await
        .unwrap()
        .unwrap();
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let reserve_stake = banks_client
        .get_account(*reserve_stake)
        .await
        .unwrap()
        .unwrap();

    let validator_sum: u64 = validator_list
        .validators
        .iter()
        .map(|info| info.stake_lamports().unwrap())
        .sum();
    let rent = banks_client.get_rent().await.unwrap();
    let rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());
    validator_sum + reserve_stake.lamports - rent - MINIMUM_RESERVE_LAMPORTS
}

pub fn add_vote_account_with_pubkey(
    voter_pubkey: &Pubkey,
    program_test: &mut ProgramTest,
) -> Pubkey {
    let authorized_voter = Pubkey::new_unique();
    let authorized_withdrawer = Pubkey::new_unique();
    let commission = 1;

    // create vote account
    let node_pubkey = Pubkey::new_unique();
    let vote_state = VoteStateVersions::new_current(VoteState::new(
        &VoteInit {
            node_pubkey,
            authorized_voter,
            authorized_withdrawer,
            commission,
        },
        &Clock::default(),
    ));
    let vote_account = SolanaAccount::create(
        ACCOUNT_RENT_EXEMPTION,
        bincode::serialize::<VoteStateVersions>(&vote_state).unwrap(),
        solana_vote_program::id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(*voter_pubkey, vote_account);
    *voter_pubkey
}

pub fn add_vote_account(program_test: &mut ProgramTest) -> Pubkey {
    let voter_pubkey = Pubkey::new_unique();
    add_vote_account_with_pubkey(&voter_pubkey, program_test)
}

#[allow(clippy::too_many_arguments)]
pub fn add_validator_stake_account(
    program_test: &mut ProgramTest,
    stake_pool: &mut state::StakePool,
    validator_list: &mut state::ValidatorList,
    stake_pool_pubkey: &Pubkey,
    withdraw_authority: &Pubkey,
    voter_pubkey: &Pubkey,
    stake_amount: u64,
    status: state::StakeStatus,
) {
    let meta = stake::state::Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: stake::state::Authorized {
            staker: *withdraw_authority,
            withdrawer: *withdraw_authority,
        },
        lockup: stake_pool.lockup,
    };

    // create validator stake account
    let stake = stake::state::Stake {
        delegation: stake::state::Delegation {
            voter_pubkey: *voter_pubkey,
            stake: stake_amount,
            activation_epoch: FIRST_NORMAL_EPOCH,
            deactivation_epoch: u64::MAX,
            ..Default::default()
        },
        credits_observed: 0,
    };

    let mut data = vec![0u8; std::mem::size_of::<stake::state::StakeStateV2>()];
    let stake_data = bincode::serialize(&stake::state::StakeStateV2::Stake(
        meta,
        stake,
        stake::stake_flags::StakeFlags::empty(),
    ))
    .unwrap();
    data[..stake_data.len()].copy_from_slice(&stake_data);
    let stake_account = SolanaAccount::create(
        stake_amount + STAKE_ACCOUNT_RENT_EXEMPTION,
        data,
        stake::program::id(),
        false,
        Epoch::default(),
    );

    let raw_suffix = 0;
    let validator_seed_suffix = NonZeroU32::new(raw_suffix);
    let (stake_address, _) = find_stake_program_address(
        &id(),
        voter_pubkey,
        stake_pool_pubkey,
        validator_seed_suffix,
    );
    program_test.add_account(stake_address, stake_account);

    let active_stake_lamports = stake_amount + STAKE_ACCOUNT_RENT_EXEMPTION;

    validator_list.validators.push(state::ValidatorStakeInfo {
        status: status.into(),
        vote_account_address: *voter_pubkey,
        active_stake_lamports: active_stake_lamports.into(),
        transient_stake_lamports: 0.into(),
        last_update_epoch: FIRST_NORMAL_EPOCH.into(),
        transient_seed_suffix: 0.into(),
        unused: 0.into(),
        validator_seed_suffix: raw_suffix.into(),
    });

    stake_pool.total_lamports += active_stake_lamports;
    stake_pool.pool_token_supply += active_stake_lamports;
}

pub fn add_reserve_stake_account(
    program_test: &mut ProgramTest,
    reserve_stake: &Pubkey,
    withdraw_authority: &Pubkey,
    stake_amount: u64,
) {
    let meta = stake::state::Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: stake::state::Authorized {
            staker: *withdraw_authority,
            withdrawer: *withdraw_authority,
        },
        lockup: stake::state::Lockup::default(),
    };
    let reserve_stake_account = SolanaAccount::create(
        stake_amount + STAKE_ACCOUNT_RENT_EXEMPTION,
        bincode::serialize::<stake::state::StakeStateV2>(&stake::state::StakeStateV2::Initialized(
            meta,
        ))
        .unwrap(),
        stake::program::id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(*reserve_stake, reserve_stake_account);
}

pub fn add_stake_pool_account(
    program_test: &mut ProgramTest,
    stake_pool_pubkey: &Pubkey,
    stake_pool: &state::StakePool,
) {
    let mut stake_pool_bytes = borsh::to_vec(&stake_pool).unwrap();
    // more room for optionals
    stake_pool_bytes.extend_from_slice(Pubkey::default().as_ref());
    stake_pool_bytes.extend_from_slice(Pubkey::default().as_ref());
    let stake_pool_account = SolanaAccount::create(
        ACCOUNT_RENT_EXEMPTION,
        stake_pool_bytes,
        id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(*stake_pool_pubkey, stake_pool_account);
}

pub fn add_validator_list_account(
    program_test: &mut ProgramTest,
    validator_list_pubkey: &Pubkey,
    validator_list: &state::ValidatorList,
    max_validators: u32,
) {
    let mut validator_list_bytes = borsh::to_vec(&validator_list).unwrap();
    // add extra room if needed
    for _ in validator_list.validators.len()..max_validators as usize {
        validator_list_bytes
            .append(&mut borsh::to_vec(&state::ValidatorStakeInfo::default()).unwrap());
    }
    let validator_list_account = SolanaAccount::create(
        ACCOUNT_RENT_EXEMPTION,
        validator_list_bytes,
        id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(*validator_list_pubkey, validator_list_account);
}

pub fn add_mint_account(
    program_test: &mut ProgramTest,
    program_id: &Pubkey,
    mint_key: &Pubkey,
    mint_authority: &Pubkey,
    supply: u64,
) {
    let mut mint_vec = vec![0u8; Mint::LEN];
    let mint = Mint {
        mint_authority: COption::Some(*mint_authority),
        supply,
        decimals: 9,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    Pack::pack(mint, &mut mint_vec).unwrap();
    let stake_pool_mint = SolanaAccount::create(
        ACCOUNT_RENT_EXEMPTION,
        mint_vec,
        *program_id,
        false,
        Epoch::default(),
    );
    program_test.add_account(*mint_key, stake_pool_mint);
}

pub fn add_token_account(
    program_test: &mut ProgramTest,
    program_id: &Pubkey,
    account_key: &Pubkey,
    mint_key: &Pubkey,
    owner: &Pubkey,
) {
    let mut fee_account_vec = vec![0u8; Account::LEN];
    let fee_account_data = Account {
        mint: *mint_key,
        owner: *owner,
        amount: 0,
        delegate: COption::None,
        state: spl_token_2022::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    Pack::pack(fee_account_data, &mut fee_account_vec).unwrap();
    let fee_account = SolanaAccount::create(
        ACCOUNT_RENT_EXEMPTION,
        fee_account_vec,
        *program_id,
        false,
        Epoch::default(),
    );
    program_test.add_account(*account_key, fee_account);
}

pub async fn setup_for_withdraw(
    token_program_id: Pubkey,
    reserve_lamports: u64,
) -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    DepositStakeAccount,
    Keypair,
    Keypair,
    u64,
) {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::new_with_token_program(token_program_id);
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            reserve_lamports,
        )
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let current_minimum_delegation = stake_pool_get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let deposit_info = simple_deposit_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        current_minimum_delegation * 3,
    )
    .await
    .unwrap();

    let tokens_to_withdraw = deposit_info.pool_tokens;

    // Delegate tokens for withdrawing
    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        tokens_to_withdraw,
    )
    .await;

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &user_stake_recipient,
    )
    .await;

    (
        context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    )
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecreaseInstruction {
    Additional,
    Reserve,
    Deprecated,
}
