#![allow(dead_code)]

use solana_program::{hash::Hash, program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use solana_vote_program::{self, vote_state::VoteState};
use spl_stake_pool::*;

const TEST_STAKE_AMOUNT: u64 = 100;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_stake_pool",
        id(),
        processor!(processor::Processor::process),
    )
}

async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    pool_mint: &Keypair,
    owner: &Pubkey,
) {
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
                &owner,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, pool_mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    owner: &Pubkey,
) {
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
                owner,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client
        .get_account(token.clone())
        .await
        .unwrap()
        .unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub async fn delegate_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    owner: &Keypair,
    delegate: &Pubkey,
    amount: u64,
) {
    let mut transaction = Transaction::new_with_payer(
        &[
            spl_token::instruction::approve(
                &spl_token::id(),
                &account,
                &delegate,
                &owner.pubkey(),
                &[],
                amount,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, owner], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

#[allow(clippy::too_many_arguments)]
async fn create_stake_pool(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool: &Keypair,
    validator_stake_list: &Keypair,
    pool_mint: &Pubkey,
    pool_token_account: &Pubkey,
    owner: &Pubkey,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(state::State::LEN);
    let rent_validator_stake_list = rent.minimum_balance(state::ValidatorStakeList::LEN);
    let numerator = 1;
    let denominator = 100;
    let fee = instruction::Fee {
        numerator,
        denominator,
    };
    let init_args = instruction::InitArgs { fee };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool.pubkey(),
                rent_stake_pool,
                state::State::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &validator_stake_list.pubkey(),
                rent_validator_stake_list,
                state::ValidatorStakeList::LEN as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool.pubkey(),
                owner,
                &validator_stake_list.pubkey(),
                pool_mint,
                pool_token_account,
                &spl_token::id(),
                init_args,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[payer, stake_pool, validator_stake_list],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn create_vote(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    vote: &Keypair,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_voter = rent.minimum_balance(VoteState::size_of());

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &vote.pubkey(),
            rent_voter,
            VoteState::size_of() as u64,
            &solana_vote_program::id(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&vote, payer], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn create_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
    authorized: &stake::Authorized,
    lockup: &stake::Lockup,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let lamports =
        rent.minimum_balance(std::mem::size_of::<stake::StakeState>()) + TEST_STAKE_AMOUNT;

    let mut transaction = Transaction::new_with_payer(
        &stake::create_account(
            &payer.pubkey(),
            &stake.pubkey(),
            authorized,
            lockup,
            lamports,
        ),
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, stake], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
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
        &[stake::delegate_stake(&stake, &authorized.pubkey(), &vote)],
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
    stake_authorize: stake::StakeAuthorize,
) {
    let mut transaction = Transaction::new_with_payer(
        &[stake::authorize(
            &stake,
            &authorized.pubkey(),
            &new_authorized,
            stake_authorize,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authorized], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub struct StakeAccount {
    pub stake_account: Keypair,
    pub target_authority: Pubkey,
    pub vote: Keypair,
}

impl StakeAccount {
    pub fn new_with_target_authority(authority: &Pubkey) -> Self {
        StakeAccount {
            stake_account: Keypair::new(),
            target_authority: *authority,
            vote: Keypair::new(),
        }
    }

    pub async fn create_and_delegate(
        &self,
        mut banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        // make stake account
        let user_stake_authority = Keypair::new();
        let lockup = stake::Lockup::default();
        let authorized = stake::Authorized {
            staker: user_stake_authority.pubkey(),
            withdrawer: self.target_authority,
        };
        create_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.stake_account,
            &authorized,
            &lockup,
        )
        .await;

        create_vote(&mut banks_client, &payer, &recent_blockhash, &self.vote).await;
        delegate_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.stake_account.pubkey(),
            &user_stake_authority,
            &self.vote.pubkey(),
        )
        .await;

        authorize_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.stake_account.pubkey(),
            &user_stake_authority,
            &self.target_authority,
            stake::StakeAuthorize::Staker,
        )
        .await;
    }
}

pub struct StakePoolAccounts {
    pub stake_pool: Keypair,
    pub validator_stake_list: Keypair,
    pub pool_mint: Keypair,
    pub pool_fee_account: Keypair,
    pub owner: Keypair,
    pub withdraw_authority: Pubkey,
    pub deposit_authority: Pubkey,
}

impl StakePoolAccounts {
    pub fn new() -> Self {
        let stake_pool = Keypair::new();
        let validator_stake_list = Keypair::new();
        let stake_pool_address = &stake_pool.pubkey();
        let (withdraw_authority, _) = Pubkey::find_program_address(
            &[&stake_pool_address.to_bytes()[..32], b"withdraw"],
            &id(),
        );
        let (deposit_authority, _) = Pubkey::find_program_address(
            &[&stake_pool_address.to_bytes()[..32], b"deposit"],
            &id(),
        );
        let pool_mint = Keypair::new();
        let pool_fee_account = Keypair::new();
        let owner = Keypair::new();

        Self {
            stake_pool,
            validator_stake_list,
            pool_mint,
            pool_fee_account,
            owner,
            withdraw_authority,
            deposit_authority,
        }
    }

    pub async fn initialize_stake_pool(
        &self,
        mut banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.pool_mint,
            &self.withdraw_authority,
        )
        .await;
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.pool_fee_account,
            &self.pool_mint.pubkey(),
            &self.owner.pubkey(),
        )
        .await;
        create_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &self.stake_pool,
            &self.validator_stake_list,
            &self.pool_mint.pubkey(),
            &self.pool_fee_account.pubkey(),
            &self.owner.pubkey(),
        )
        .await;
    }

    pub async fn deposit_stake(
        &self,
        stake: &Pubkey,
        pool_account: &Pubkey,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::deposit(
                &id(),
                &self.stake_pool.pubkey(),
                &self.deposit_authority,
                &self.withdraw_authority,
                stake,
                pool_account,
                &self.pool_fee_account.pubkey(),
                &self.pool_mint.pubkey(),
                &spl_token::id(),
                &stake::id(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
    }

    pub async fn join_pool(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake: &Pubkey,
        pool_account: &Pubkey,
    ) -> Option<TransportError> {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::join_pool(
                &id(),
                &self.stake_pool.pubkey(),
                &self.owner.pubkey(),
                &self.deposit_authority,
                &self.withdraw_authority,
                &self.validator_stake_list.pubkey(),
                stake,
                pool_account,
                &self.pool_mint.pubkey(),
                &spl_token::id(),
                &stake::id(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer, &self.owner], *recent_blockhash);
        banks_client.process_transaction(transaction).await.err()
    }

    pub async fn leave_pool(
        &self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        stake: &Pubkey,
        pool_account: &Pubkey,
        new_authority: &Pubkey,
    ) -> Option<TransportError> {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::leave_pool(
                &id(),
                &self.stake_pool.pubkey(),
                &self.owner.pubkey(),
                &self.withdraw_authority,
                &new_authority,
                &self.validator_stake_list.pubkey(),
                stake,
                pool_account,
                &self.pool_mint.pubkey(),
                &spl_token::id(),
                &stake::id(),
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer, &self.owner], *recent_blockhash);
        banks_client.process_transaction(transaction).await.err()
    }
}
