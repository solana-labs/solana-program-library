//! Instruction types

#![allow(clippy::too_many_arguments)]

use {
    crate::{
        find_default_deposit_account_address_and_seed, find_pool_address, find_pool_mint_address,
        find_pool_mint_authority_address, find_pool_mpl_authority_address, find_pool_stake_address,
        find_pool_stake_authority_address,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    mpl_token_metadata::pda::find_metadata_account,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        stake, system_instruction, system_program, sysvar,
    },
};

/// Instructions supported by the SinglePool program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum SinglePoolInstruction {
    ///   Initialize the mint and stake account for a new single-validator pool.
    ///   The pool stake account must contain the rent-exempt minimum plus the minimum delegation.
    ///   No tokens will be minted: to deposit more, use `Deposit` after `InitializeStake`.
    ///
    ///   0. `[]` Validator vote account
    ///   1. `[w]` Pool account
    ///   2. `[w]` Pool stake account
    ///   3. `[w]` Pool token mint
    ///   4. `[]` Pool stake authority
    ///   5. `[]` Pool mint authority
    ///   6. `[]` Rent sysvar
    ///   7. `[]` Clock sysvar
    ///   8. `[]` Stake history sysvar
    ///   9. `[]` Stake config sysvar
    ///  10. `[]` System program
    ///  11. `[]` Token program
    ///  12. `[]` Stake program
    InitializePool,

    // TODO all other ixns
    ///   Deposit stake into the pool.  The output is a "pool" token representing fractional
    ///   ownership of the pool stake. Inputs are converted to the current ratio.
    ///
    ///   0. `[w]` Pool stake account
    ///   1. `[w]` Pool token mint
    ///   2. `[]` Pool stake authority
    ///   3. `[]` Pool mint authority
    ///   4. `[w]` User stake account to join to the pool
    ///   5. `[w]` User account to receive pool tokens
    ///   6. `[w]` User account to receive lamports
    ///   7. `[]` Clock sysvar
    ///   8. `[]` Stake history sysvar
    ///   9. `[]` Token program
    ///  10. `[]` Stake program
    DepositStake {
        // TODO remove
        /// Validator vote account address
        vote_account_address: Pubkey,
    },

    ///   Redeem tokens issued by this pool for stake at the current ratio.
    ///
    ///   0. `[w]` Pool stake account
    ///   1. `[w]` Pool token mint
    ///   2. `[]` Pool stake authority
    ///   3. `[]` Pool mint authority
    ///   4. `[w]` User stake account to receive stake at
    ///   5. `[w]` User account to take pool tokens from
    ///   6. `[]` Clock sysvar
    ///   7. `[]` Token program
    ///   8. `[]` Stake program
    WithdrawStake {
        // TODO remove
        /// Validator vote account address
        vote_account_address: Pubkey,
        /// User authority for the new stake account
        user_stake_authority: Pubkey,
        /// Amount of tokens to redeem for stake
        token_amount: u64,
    },

    ///   Create token metadata for the stake-pool token in the metaplex-token program.
    ///   Step three of the permissionless three-stage initialization flow.
    ///   Note this instruction is not necessary for the pool to operate, to ensure we cannot
    ///   be broken by upstream.
    ///
    ///   0. `[]` Pool token mint
    ///   1. `[]` Pool mint authority
    ///   2. `[]` Pool MPL authority
    ///   3. `[s, w]` Payer for creation of token metadata account
    ///   4. `[w]` Token metadata account
    ///   5. `[]` Metadata program id
    ///   6. `[]` System program id
    CreateTokenMetadata {
        // TODO remove
        /// Validator vote account address
        vote_account_address: Pubkey,
    },

    ///   Update token metadata for the stake-pool token in the metaplex-token program.
    ///
    ///   0. `[]` Validator vote account
    ///   1. `[]` Pool MPL authority
    ///   2. `[s]` Vote account authorized withdrawer
    ///   3. `[w]` Token metadata account
    ///   4. `[]` Metadata program id
    UpdateTokenMetadata {
        /// Token name
        name: String,
        /// Token symbol e.g. stkSOL
        symbol: String,
        /// URI of the uploaded metadata of the spl-token
        uri: String,
    },
}

// XXX TODO FIXME initialize takes vote account
// all other fuctions take pool account. all instructions take pool account and drop vote from structs
// process_initialize needs to alloc the struct and store the vote account
// process_deposit/withdraw need to deserialize and get vote account (do they? is that true?)
// use it for check_ functions. also check_ functions must be updated
// process_update needs vote account for confirming the validator one is right
// uhhh then i need to refactor tests kms

/// Creates all necessary instructions to initialize the stake pool.
pub fn initialize(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    payer: &Pubkey,
    rent: &Rent,
    minimum_delegation: u64,
) -> Vec<Instruction> {
    let pool_address = find_pool_address(program_id, vote_account_address);
    let stake_address = find_pool_stake_address(program_id, &pool_address);
    let stake_space = std::mem::size_of::<stake::state::StakeState>();
    let stake_rent_plus_minimum = rent
        .minimum_balance(stake_space)
        .saturating_add(minimum_delegation);

    let mint_address = find_pool_mint_address(program_id, &pool_address);
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    vec![
        system_instruction::transfer(payer, &stake_address, stake_rent_plus_minimum),
        system_instruction::transfer(payer, &mint_address, mint_rent),
        initialize_pool(program_id, vote_account_address),
        // TODO pool?
        create_token_metadata(program_id, vote_account_address, payer),
    ]
}

/// Creates an `InitializePool` instruction.
pub fn initialize_pool(program_id: &Pubkey, vote_account_address: &Pubkey) -> Instruction {
    let pool_address = find_pool_address(program_id, vote_account_address);
    let mint_address = find_pool_mint_address(program_id, &pool_address);

    let data = SinglePoolInstruction::InitializePool.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(*vote_account_address, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(find_pool_stake_address(program_id, &pool_address), false),
        AccountMeta::new(mint_address, false),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, &pool_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, &pool_address),
            false,
        ),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

// TODO everything below

/// Creates all necessary instructions to deposit stake.
pub fn deposit(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_token_account: &Pubkey,
    user_lamport_account: &Pubkey,
    user_withdraw_authority: &Pubkey,
) -> Vec<Instruction> {
    let pool_stake_authority = find_pool_stake_authority_address(program_id, vote_account_address);

    vec![
        stake::instruction::authorize(
            user_stake_account,
            user_withdraw_authority,
            &pool_stake_authority,
            stake::state::StakeAuthorize::Staker,
            None,
        ),
        stake::instruction::authorize(
            user_stake_account,
            user_withdraw_authority,
            &pool_stake_authority,
            stake::state::StakeAuthorize::Withdrawer,
            None,
        ),
        deposit_stake(
            program_id,
            vote_account_address,
            user_stake_account,
            user_token_account,
            user_lamport_account,
        ),
    ]
}

/// Creates a `DepositStake` instruction.
pub fn deposit_stake(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_token_account: &Pubkey,
    user_lamport_account: &Pubkey,
) -> Instruction {
    let data = SinglePoolInstruction::DepositStake {
        vote_account_address: *vote_account_address,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(
            find_pool_stake_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new(
            find_pool_mint_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new(*user_stake_account, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(*user_lamport_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates all necessary instructions to withdraw stake into a given stake account.
/// If a new stake account is required, the user should first include `system_instruction::create_account`
/// with account size `std::mem::size_of::<stake::state::StakeState>()` and owner `stake::program::id()`.
pub fn withdraw(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_stake_authority: &Pubkey,
    user_token_account: &Pubkey,
    user_token_authority: &Pubkey,
    token_amount: u64,
) -> Vec<Instruction> {
    vec![
        spl_token::instruction::approve(
            &spl_token::id(),
            user_token_account,
            &find_pool_mint_authority_address(program_id, vote_account_address),
            user_token_authority,
            &[],
            token_amount,
        )
        .unwrap(),
        withdraw_stake(
            program_id,
            vote_account_address,
            user_stake_account,
            user_stake_authority,
            user_token_account,
            token_amount,
        ),
    ]
}

/// Creates a `WithdrawStake` instruction.
pub fn withdraw_stake(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_stake_authority: &Pubkey,
    user_token_account: &Pubkey,
    token_amount: u64,
) -> Instruction {
    let data = SinglePoolInstruction::WithdrawStake {
        vote_account_address: *vote_account_address,
        user_stake_authority: *user_stake_authority,
        token_amount,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new(
            find_pool_stake_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new(
            find_pool_mint_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new(*user_stake_account, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates necessary instructions to create and delegate a new stake account to a given validator.
/// Uses a fixed address for each wallet and vote account combination to make it easier to find for deposits.
/// This is an optional helper function; deposits can come from any owned stake account without lockup.
pub fn create_and_delegate_user_stake(
    vote_account_address: &Pubkey,
    user_wallet: &Pubkey,
    rent: &Rent,
    stake_amount: u64,
) -> Vec<Instruction> {
    let stake_space = std::mem::size_of::<stake::state::StakeState>();
    let lamports = rent
        .minimum_balance(stake_space)
        .saturating_add(stake_amount);
    let (deposit_address, deposit_seed) =
        find_default_deposit_account_address_and_seed(vote_account_address, user_wallet);

    stake::instruction::create_account_with_seed_and_delegate_stake(
        user_wallet,
        &deposit_address,
        user_wallet,
        &deposit_seed,
        vote_account_address,
        &stake::state::Authorized::auto(user_wallet),
        &stake::state::Lockup::default(),
        lamports,
    )
}

/// Creates a `CreateTokenMetadata` instruction.
pub fn create_token_metadata(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let pool_mint = find_pool_mint_address(program_id, vote_account_address);
    let (token_metadata, _) = find_metadata_account(&pool_mint);
    let data = SinglePoolInstruction::CreateTokenMetadata {
        vote_account_address: *vote_account_address,
    }
    .try_to_vec()
    .unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(pool_mint, false),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mpl_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new(*payer, true),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(mpl_token_metadata::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates an `UpdateTokenMetadata` instruction.
pub fn update_token_metadata(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    authorized_withdrawer: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let pool_mint = find_pool_mint_address(program_id, vote_account_address);
    let (token_metadata, _) = find_metadata_account(&pool_mint);
    let data = SinglePoolInstruction::UpdateTokenMetadata { name, symbol, uri }
        .try_to_vec()
        .unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(*vote_account_address, false),
        AccountMeta::new_readonly(
            find_pool_mpl_authority_address(program_id, vote_account_address),
            false,
        ),
        AccountMeta::new_readonly(*authorized_withdrawer, true),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(mpl_token_metadata::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}
