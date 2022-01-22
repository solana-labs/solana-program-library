#![allow(dead_code)]

use {
    solana_program::{
        borsh::{get_instance_packed_len, get_packed_len, try_from_slice_unchecked},
        hash::Hash,
        program_pack::Pack,
        pubkey::Pubkey,
        stake, system_instruction, system_program,
    },
    solana_program_test::*,
    solana_sdk::{
        account::Account,
        signature::{Keypair, Signer},
        transaction::Transaction,
        transport::TransportError,
    },
    solana_vote_program::{
        self, vote_instruction,
        vote_state::{VoteInit, VoteState},
    },
    spl_stake_pool::{
        find_deposit_authority_program_address, find_stake_program_address,
        find_transient_stake_program_address, find_withdraw_authority_program_address, id,
        instruction, processor,
        state::{self, FeeType, ValidatorList},
        MINIMUM_ACTIVE_STAKE,
    },
};

pub const TEST_STAKE_AMOUNT: u64 = 1_500_000_000;
pub const MAX_TEST_VALIDATORS: u32 = 10_000;
pub const DEFAULT_TRANSIENT_STAKE_SEED: u64 = 42;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_stake_pool",
        id(),
        processor!(processor::Processor::process),
    )
}

pub async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> Account {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    pool_mint: &Keypair,
    manager: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &pool_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &pool_mint.pubkey(),
                manager,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, pool_mint], *recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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

pub async fn transfer_spl_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            source,
            destination,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    manager: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                pool_mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn close_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    lamports_destination: &Pubkey,
    manager: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::close_account(
            &spl_token::id(),
            account,
            lamports_destination,
            &manager.pubkey(),
            &[],
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, manager], *recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn freeze_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    pool_mint: &Pubkey,
    manager: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::freeze_account(
            &spl_token::id(),
            account,
            pool_mint,
            &manager.pubkey(),
            &[],
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, manager], *recent_blockhash);
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
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
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn burn_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::burn(
            &spl_token::id(),
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
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    banks_client
        .process_transaction(transaction)
        .await
        .map_err(|e| e.into())
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub async fn get_token_supply(banks_client: &mut BanksClient, mint: &Pubkey) -> u64 {
    let mint_account = banks_client.get_account(*mint).await.unwrap().unwrap();
    let account_info =
        spl_token::state::Mint::unpack_from_slice(mint_account.data.as_slice()).unwrap();
    account_info.supply
}

pub async fn delegate_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    manager: &Keypair,
    delegate: &Pubkey,
    amount: u64,
) {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::approve(
            &spl_token::id(),
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

#[allow(clippy::too_many_arguments)]
pub async fn create_stake_pool(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool: &Keypair,
    validator_list: &Keypair,
    reserve_stake: &Pubkey,
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
                &spl_token::id(),
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
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
    instructions.append(&mut vote_instruction::create_account(
        &payer.pubkey(),
        &vote.pubkey(),
        &VoteInit {
            node_pubkey: validator.pubkey(),
            authorized_voter: validator.pubkey(),
            ..VoteInit::default()
        },
        rent_voter,
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
        rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>()) + stake_amount;

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
    let lamports = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>()) + 1;

    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake.pubkey(),
            lamports,
            std::mem::size_of::<stake::state::StakeState>() as u64,
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
) -> ValidatorStakeAccount {
    let mut unknown_stake = ValidatorStakeAccount::new(stake_pool, 222);
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
        MINIMUM_ACTIVE_STAKE,
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
    pub vote: Keypair,
    pub validator: Keypair,
    pub stake_pool: Pubkey,
}

impl ValidatorStakeAccount {
    pub fn new(stake_pool: &Pubkey, transient_stake_seed: u64) -> Self {
        let validator = Keypair::new();
        let vote = Keypair::new();
        let (stake_account, _) = find_stake_program_address(&id(), &vote.pubkey(), stake_pool);
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
    pub pool_mint: Keypair,
    pub pool_fee_account: Keypair,
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
}

impl StakePoolAccounts {
    pub fn new() -> Self {
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
            pool_mint,
            pool_fee_account,
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
        }
    }

    pub fn new_with_deposit_authority(stake_deposit_authority: Keypair) -> Self {
        let mut stake_pool_accounts = Self::new();
        stake_pool_accounts.stake_deposit_authority = stake_deposit_authority.pubkey();
        stake_pool_accounts.stake_deposit_authority_keypair = Some(stake_deposit_authority);
        stake_pool_accounts
    }

    pub fn calculate_fee(&self, amount: u64) -> u64 {
        amount * self.epoch_fee.numerator / self.epoch_fee.denominator
    }

    pub fn calculate_withdrawal_fee(&self, pool_tokens: u64) -> u64 {
        pool_tokens * self.withdrawal_fee.numerator / self.withdrawal_fee.denominator
    }

    pub fn calculate_referral_fee(&self, deposit_fee_collected: u64) -> u64 {
        deposit_fee_collected * self.referral_fee as u64 / 100
    }

    pub fn calculate_sol_deposit_fee(&self, pool_tokens: u64) -> u64 {
        pool_tokens * self.sol_deposit_fee.numerator / self.sol_deposit_fee.denominator
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
            &self.pool_mint,
            &self.withdraw_authority,
        )
        .await?;
        create_token_account(
            banks_client,
            payer,
            recent_blockhash,
            &self.pool_fee_account,
            &self.pool_mint.pubkey(),
            &self.manager.pubkey(),
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
        let instructions =
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
                    &spl_token::id(),
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
                    &spl_token::id(),
                )
            };
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer.pubkey()),
            &signers,
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
                &spl_token::id(),
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
                &spl_token::id(),
                amount,
            )
        };
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &signers,
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::withdraw_stake(
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
                &spl_token::id(),
                amount,
            )],
            Some(&payer.pubkey()),
            &[payer, user_transfer_authority],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
                &spl_token::id(),
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
                &spl_token::id(),
                amount,
            )
        };
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &signers,
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
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
        validator_vote_accounts: &[Pubkey],
        no_merge: bool,
    ) -> Option<TransportError> {
        let validator_list = self.get_validator_list(banks_client).await;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::update_validator_list_balance(
                &id(),
                &self.stake_pool.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                &self.reserve_stake.pubkey(),
                &validator_list,
                validator_vote_accounts,
                0,
                no_merge,
            )],
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::update_stake_pool_balance(
                &id(),
                &self.stake_pool.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                &self.reserve_stake.pubkey(),
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &spl_token::id(),
            )],
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::cleanup_removed_validator_entries(
                &id(),
                &self.stake_pool.pubkey(),
                &self.validator_list.pubkey(),
            )],
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
        validator_vote_accounts: &[Pubkey],
        no_merge: bool,
    ) -> Option<TransportError> {
        let validator_list = self.get_validator_list(banks_client).await;
        let transaction = Transaction::new_signed_with_payer(
            &[
                instruction::update_validator_list_balance(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.withdraw_authority,
                    &self.validator_list.pubkey(),
                    &self.reserve_stake.pubkey(),
                    &validator_list,
                    validator_vote_accounts,
                    0,
                    no_merge,
                ),
                instruction::update_stake_pool_balance(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.withdraw_authority,
                    &self.validator_list.pubkey(),
                    &self.reserve_stake.pubkey(),
                    &self.pool_fee_account.pubkey(),
                    &self.pool_mint.pubkey(),
                    &spl_token::id(),
                ),
                instruction::cleanup_removed_validator_entries(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.validator_list.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
            &[payer],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
    ) -> Option<TransportError> {
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::add_validator_to_pool(
                &id(),
                &self.stake_pool.pubkey(),
                &self.staker.pubkey(),
                &payer.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                stake,
                validator,
            )],
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
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
        new_authority: &Pubkey,
        validator_stake: &Pubkey,
        transient_stake: &Pubkey,
        destination_stake: &Keypair,
    ) -> Option<TransportError> {
        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &destination_stake.pubkey(),
                    0,
                    std::mem::size_of::<stake::state::StakeState>() as u64,
                    &stake::program::id(),
                ),
                instruction::remove_validator_from_pool(
                    &id(),
                    &self.stake_pool.pubkey(),
                    &self.staker.pubkey(),
                    &self.withdraw_authority,
                    new_authority,
                    &self.validator_list.pubkey(),
                    validator_stake,
                    transient_stake,
                    &destination_stake.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
            &[payer, &self.staker, destination_stake],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn decrease_validator_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_stake: &Pubkey,
        transient_stake: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
    ) -> Option<TransportError> {
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::decrease_validator_stake(
                &id(),
                &self.stake_pool.pubkey(),
                &self.staker.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                validator_stake,
                transient_stake,
                lamports,
                transient_stake_seed,
            )],
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn increase_validator_stake(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        transient_stake: &Pubkey,
        validator: &Pubkey,
        lamports: u64,
        transient_stake_seed: u64,
    ) -> Option<TransportError> {
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::increase_validator_stake(
                &id(),
                &self.stake_pool.pubkey(),
                &self.staker.pubkey(),
                &self.withdraw_authority,
                &self.validator_list.pubkey(),
                &self.reserve_stake.pubkey(),
                transient_stake,
                validator,
                lamports,
                transient_stake_seed,
            )],
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }

    pub async fn set_preferred_validator(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        validator_type: instruction::PreferredValidatorType,
        validator: Option<Pubkey>,
    ) -> Option<TransportError> {
        let transaction = Transaction::new_signed_with_payer(
            &[instruction::set_preferred_validator(
                &id(),
                &self.stake_pool.pubkey(),
                &self.staker.pubkey(),
                &self.validator_list.pubkey(),
                validator_type,
                validator,
            )],
            Some(&payer.pubkey()),
            &[payer, &self.staker],
            *recent_blockhash,
        );
        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
        banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| e.into())
            .err()
    }
}

pub async fn simple_add_validator_to_pool(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool_accounts: &StakePoolAccounts,
) -> ValidatorStakeAccount {
    let validator_stake = ValidatorStakeAccount::new(
        &stake_pool_accounts.stake_pool.pubkey(),
        DEFAULT_TRANSIENT_STAKE_SEED,
    );

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
        )
        .await;
    assert!(error.is_none());

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
            &self.pool_account,
            &stake_pool_accounts.pool_mint.pubkey(),
            &self.authority.pubkey(),
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
        assert!(error.is_none());
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
        &pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &authority.pubkey(),
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
        .map(|info| info.stake_lamports())
        .sum();
    let rent = banks_client.get_rent().await.unwrap();
    let rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());
    validator_sum + reserve_stake.lamports - rent - 1
}
