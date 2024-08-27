#![allow(dead_code)] // needed because cargo doesn't understand test usage

use {
    solana_program_test::*,
    solana_sdk::{
        account::Account as SolanaAccount,
        feature_set::stake_raise_minimum_delegation_to_1_sol,
        hash::Hash,
        program_error::ProgramError,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        stake::state::{Authorized, Lockup},
        system_instruction, system_program,
        transaction::{Transaction, TransactionError},
    },
    solana_vote_program::{
        self, vote_instruction,
        vote_state::{VoteInit, VoteState},
    },
    spl_associated_token_account_client::address as atoken,
    spl_single_pool::{
        find_pool_address, find_pool_mint_address, find_pool_mint_authority_address,
        find_pool_mpl_authority_address, find_pool_stake_address,
        find_pool_stake_authority_address, id, inline_mpl_token_metadata, instruction,
        processor::Processor,
    },
};

pub mod token;
pub use token::*;

pub mod stake;
pub use stake::*;

pub const FIRST_NORMAL_EPOCH: u64 = 15;
pub const USER_STARTING_LAMPORTS: u64 = 10_000_000_000_000; // 10k sol

pub fn program_test(enable_minimum_delegation: bool) -> ProgramTest {
    let mut program_test = ProgramTest::default();

    program_test.add_program("mpl_token_metadata", inline_mpl_token_metadata::id(), None);
    program_test.add_program("spl_single_pool", id(), processor!(Processor::process));
    program_test.prefer_bpf(false);

    if !enable_minimum_delegation {
        program_test.deactivate_feature(stake_raise_minimum_delegation_to_1_sol::id());
    }

    program_test
}

#[derive(Debug, PartialEq)]
pub struct SinglePoolAccounts {
    pub validator: Keypair,
    pub voter: Keypair,
    pub withdrawer: Keypair,
    pub vote_account: Keypair,
    pub pool: Pubkey,
    pub stake_account: Pubkey,
    pub mint: Pubkey,
    pub stake_authority: Pubkey,
    pub mint_authority: Pubkey,
    pub mpl_authority: Pubkey,
    pub alice: Keypair,
    pub bob: Keypair,
    pub alice_stake: Keypair,
    pub bob_stake: Keypair,
    pub alice_token: Pubkey,
    pub bob_token: Pubkey,
    pub token_program_id: Pubkey,
}
impl SinglePoolAccounts {
    // does everything in initialize_for_deposit plus performs the deposit(s) and
    // creates blank account(s) optionally advances to activation before the
    // deposit
    pub async fn initialize_for_withdraw(
        &self,
        context: &mut ProgramTestContext,
        alice_amount: u64,
        maybe_bob_amount: Option<u64>,
        activate: bool,
    ) -> u64 {
        let minimum_delegation = self
            .initialize_for_deposit(context, alice_amount, maybe_bob_amount)
            .await;

        if activate {
            advance_epoch(context).await;
        }

        let instructions = instruction::deposit(
            &id(),
            &self.pool,
            &self.alice_stake.pubkey(),
            &self.alice_token,
            &self.alice.pubkey(),
            &self.alice.pubkey(),
        );
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.alice],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        create_blank_stake_account(
            &mut context.banks_client,
            &context.payer,
            &self.alice,
            &context.last_blockhash,
            &self.alice_stake,
        )
        .await;

        if maybe_bob_amount.is_some() {
            let instructions = instruction::deposit(
                &id(),
                &self.pool,
                &self.bob_stake.pubkey(),
                &self.bob_token,
                &self.bob.pubkey(),
                &self.bob.pubkey(),
            );
            let transaction = Transaction::new_signed_with_payer(
                &instructions,
                Some(&context.payer.pubkey()),
                &[&context.payer, &self.bob],
                context.last_blockhash,
            );

            context
                .banks_client
                .process_transaction(transaction)
                .await
                .unwrap();

            create_blank_stake_account(
                &mut context.banks_client,
                &context.payer,
                &self.bob,
                &context.last_blockhash,
                &self.bob_stake,
            )
            .await;
        }

        minimum_delegation
    }

    // does everything in initialize plus creates/delegates one or both stake
    // accounts for our users note this does not advance time, so everything is
    // in an activating state
    pub async fn initialize_for_deposit(
        &self,
        context: &mut ProgramTestContext,
        alice_amount: u64,
        maybe_bob_amount: Option<u64>,
    ) -> u64 {
        let minimum_delegation = self.initialize(context).await;

        create_independent_stake_account(
            &mut context.banks_client,
            &context.payer,
            &self.alice,
            &context.last_blockhash,
            &self.alice_stake,
            &Authorized::auto(&self.alice.pubkey()),
            &Lockup::default(),
            alice_amount,
        )
        .await;

        delegate_stake_account(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &self.alice_stake.pubkey(),
            &self.alice,
            &self.vote_account.pubkey(),
        )
        .await;

        if let Some(bob_amount) = maybe_bob_amount {
            create_independent_stake_account(
                &mut context.banks_client,
                &context.payer,
                &self.bob,
                &context.last_blockhash,
                &self.bob_stake,
                &Authorized::auto(&self.bob.pubkey()),
                &Lockup::default(),
                bob_amount,
            )
            .await;

            delegate_stake_account(
                &mut context.banks_client,
                &context.payer,
                &context.last_blockhash,
                &self.bob_stake.pubkey(),
                &self.bob,
                &self.vote_account.pubkey(),
            )
            .await;
        };

        minimum_delegation
    }

    // creates a vote account and stake pool for it. also sets up two users with sol
    // and token accounts note this leaves the pool in an activating state.
    // caller can advance to next epoch if they please
    pub async fn initialize(&self, context: &mut ProgramTestContext) -> u64 {
        let slot = context.genesis_config().epoch_schedule.first_normal_slot + 1;
        context.warp_to_slot(slot).unwrap();

        create_vote(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &self.validator,
            &self.voter.pubkey(),
            &self.withdrawer.pubkey(),
            &self.vote_account,
        )
        .await;

        let rent = context.banks_client.get_rent().await.unwrap();
        let minimum_delegation = get_pool_minimum_delegation(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;

        let instructions = instruction::initialize(
            &id(),
            &self.vote_account.pubkey(),
            &context.payer.pubkey(),
            &rent,
            minimum_delegation,
        );
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &self.alice.pubkey(),
            USER_STARTING_LAMPORTS,
        )
        .await;

        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &self.bob.pubkey(),
            USER_STARTING_LAMPORTS,
        )
        .await;

        create_ata(
            &mut context.banks_client,
            &context.payer,
            &self.alice.pubkey(),
            &context.last_blockhash,
            &self.mint,
        )
        .await;

        create_ata(
            &mut context.banks_client,
            &context.payer,
            &self.bob.pubkey(),
            &context.last_blockhash,
            &self.mint,
        )
        .await;

        minimum_delegation
    }
}
impl Default for SinglePoolAccounts {
    fn default() -> Self {
        let vote_account = Keypair::new();
        let alice = Keypair::new();
        let bob = Keypair::new();
        let pool = find_pool_address(&id(), &vote_account.pubkey());
        let mint = find_pool_mint_address(&id(), &pool);

        Self {
            validator: Keypair::new(),
            voter: Keypair::new(),
            withdrawer: Keypair::new(),
            stake_account: find_pool_stake_address(&id(), &pool),
            pool,
            mint,
            stake_authority: find_pool_stake_authority_address(&id(), &pool),
            mint_authority: find_pool_mint_authority_address(&id(), &pool),
            mpl_authority: find_pool_mpl_authority_address(&id(), &pool),
            vote_account,
            alice_stake: Keypair::new(),
            bob_stake: Keypair::new(),
            alice_token: atoken::get_associated_token_address(&alice.pubkey(), &mint),
            bob_token: atoken::get_associated_token_address(&bob.pubkey(), &mint),
            alice,
            bob,
            token_program_id: spl_token::id(),
        }
    }
}

pub async fn refresh_blockhash(context: &mut ProgramTestContext) {
    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
}

pub async fn advance_epoch(context: &mut ProgramTestContext) {
    let root_slot = context.banks_client.get_root_slot().await.unwrap();
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    context.warp_to_slot(root_slot + slots_per_epoch).unwrap();
}

pub async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> SolanaAccount {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("client error")
        .expect("account not found")
}

pub async fn create_vote(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    validator: &Keypair,
    voter: &Pubkey,
    withdrawer: &Pubkey,
    vote_account: &Keypair,
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
        &vote_account.pubkey(),
        &VoteInit {
            node_pubkey: validator.pubkey(),
            authorized_voter: *voter,
            authorized_withdrawer: *withdrawer,
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
        &[validator, vote_account, payer],
        *recent_blockhash,
    );

    // ignore errors for idempotency
    let _ = banks_client.process_transaction(transaction).await;
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

pub fn check_error<T: Clone + std::fmt::Debug>(got: BanksClientError, expected: T)
where
    ProgramError: TryFrom<T>,
{
    // banks error -> transaction error -> instruction error -> program error
    let got_p: ProgramError = if let TransactionError::InstructionError(_, e) = got.unwrap() {
        e.try_into().unwrap()
    } else {
        panic!(
            "couldn't convert {:?} to ProgramError (expected {:?})",
            got, expected
        );
    };

    // this silly thing is because we can guarantee From<T> has a Debug for T
    // but TryFrom<T> produces Result<T, E> and E may not have Debug. so we can't
    // call unwrap also we use TryFrom because we have to go `instruction
    // error-> program error` because StakeError impls the former but not the
    // latter... and that conversion is merely surjective........
    // infomercial lady: "if only there were a better way!"
    let expected_p = match expected.clone().try_into() {
        Ok(v) => v,
        Err(_) => panic!("could not unwrap {:?}", expected),
    };

    if got_p != expected_p {
        panic!(
            "error comparison failed!\n\nGOT: {:#?} / ({:?})\n\nEXPECTED: {:#?} / ({:?})\n\n",
            got, got_p, expected, expected_p
        );
    }
}
