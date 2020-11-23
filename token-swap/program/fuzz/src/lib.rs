//! Helpers for working with swaps in a fuzzing environment

use spl_token_swap::{
    curve::base::SwapCurve,
    instruction::{self, Deposit, Swap, Withdraw},
    state::SwapInfo,
};

use spl_token::{
    instruction::approve,
    state::{Account as TokenAccount, AccountState as TokenAccountState, Mint},
};

use solana_program::{
    account_info::AccountInfo, bpf_loader, clock::Epoch, entrypoint::ProgramResult,
    instruction::Instruction, program_error::ProgramError, program_option::COption,
    program_pack::Pack, program_stubs, pubkey::Pubkey, system_program,
};

struct TestSyscallStubs {}
impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let mut new_account_infos = vec![];

        // mimic check for token program in accounts
        if !account_infos.iter().any(|x| *x.key == spl_token::id()) {
            return Err(ProgramError::InvalidAccountData);
        }

        for meta in instruction.accounts.iter() {
            for account_info in account_infos.iter() {
                if meta.pubkey == *account_info.key {
                    let mut new_account_info = account_info.clone();
                    for seeds in signers_seeds.iter() {
                        let signer =
                            Pubkey::create_program_address(&seeds, &spl_token_swap::id()).unwrap();
                        if *account_info.key == signer {
                            new_account_info.is_signer = true;
                        }
                    }
                    new_account_infos.push(new_account_info);
                }
            }
        }

        spl_token::processor::Processor::process(
            &instruction.program_id,
            &new_account_infos,
            &instruction.data,
        )
    }
}

fn test_syscall_stubs() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
    });
}

fn do_process_instruction(instruction: Instruction, accounts: &[AccountInfo]) -> ProgramResult {
    test_syscall_stubs();

    // approximate the logic in the actual runtime which runs the instruction
    // and only updates accounts if the instruction is successful
    let mut account_data = accounts
        .iter()
        .map(AccountData::new_from_account_info)
        .collect::<Vec<_>>();
    let account_infos = account_data
        .iter_mut()
        .map(AccountData::into_account_info)
        .collect::<Vec<_>>();
    let res = if instruction.program_id == spl_token_swap::id() {
        spl_token_swap::processor::Processor::process(
            &instruction.program_id,
            &account_infos,
            &instruction.data,
        )
    } else {
        spl_token::processor::Processor::process(
            &instruction.program_id,
            &account_infos,
            &instruction.data,
        )
    };

    if res.is_ok() {
        let mut account_metas = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account))
            .collect::<Vec<_>>();
        for account_info in account_infos.iter() {
            for account_meta in account_metas.iter_mut() {
                if account_info.key == account_meta.0 {
                    let account = &mut account_meta.1;
                    let mut lamports = account.lamports.borrow_mut();
                    **lamports = **account_info.lamports.borrow();
                    let mut data = account.data.borrow_mut();
                    data.clone_from_slice(*account_info.data.borrow());
                }
            }
        }
    }
    res
}

#[derive(Clone)]
pub struct AccountData {
    pub key: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub program_id: Pubkey,
    pub is_signer: bool,
}

impl AccountData {
    pub fn new(size: usize, program_id: Pubkey) -> Self {
        Self {
            key: Pubkey::new_unique(),
            lamports: 0,
            data: vec![0; size],
            program_id,
            is_signer: false,
        }
    }

    pub fn new_from_account_info(account_info: &AccountInfo) -> Self {
        Self {
            key: account_info.key.clone(),
            lamports: account_info.lamports.borrow().clone(),
            data: account_info.data.borrow().to_vec(),
            program_id: account_info.owner.clone(),
            is_signer: account_info.is_signer,
        }
    }

    pub fn into_account_info(&mut self) -> AccountInfo {
        AccountInfo::new(
            &self.key,
            self.is_signer,
            false,
            &mut self.lamports,
            &mut self.data[..],
            &self.program_id,
            false,
            Epoch::default(),
        )
    }
}

pub struct TokenSwapAccountInfo {
    pub user_account: AccountData,
    pub nonce: u8,
    pub authority_account: AccountData,
    pub swap_curve: SwapCurve,
    pub swap_account: AccountData,
    pub pool_mint_account: AccountData,
    pub pool_fee_account: AccountData,
    pub pool_token_account: AccountData,
    pub token_a_account: AccountData,
    pub token_a_mint_account: AccountData,
    pub token_b_account: AccountData,
    pub token_b_mint_account: AccountData,
    pub token_program_account: AccountData,
}

pub fn create_mint(owner: &Pubkey) -> AccountData {
    let mut account_data = AccountData::new(Mint::LEN, spl_token::id());
    let mut mint = Mint::default();
    mint.is_initialized = true;
    mint.mint_authority = COption::Some(*owner);
    Mint::pack(mint, &mut account_data.data[..]).unwrap();
    account_data
}

pub fn create_token_account(
    mint_account: &mut AccountData,
    owner: &Pubkey,
    amount: u64,
) -> AccountData {
    let mut mint = Mint::unpack(&mint_account.data).unwrap();
    let mut account_data = AccountData::new(TokenAccount::LEN, spl_token::id());
    let mut account = TokenAccount::default();
    account.state = TokenAccountState::Initialized;
    account.mint = mint_account.key.clone();
    account.owner = *owner;
    account.amount = amount;
    mint.supply += amount;
    Mint::pack(mint, &mut mint_account.data[..]).unwrap();
    TokenAccount::pack(account, &mut account_data.data[..]).unwrap();
    account_data
}

pub fn get_token_balance(account_data: &AccountData) -> u64 {
    let account = TokenAccount::unpack(&account_data.data).unwrap();
    account.amount
}

pub fn create_program_account(program_id: Pubkey) -> AccountData {
    let mut account_data = AccountData::new(0, bpf_loader::id());
    account_data.key = program_id;
    account_data
}

impl TokenSwapAccountInfo {
    pub fn new(swap_curve: SwapCurve, token_a_amount: u64, token_b_amount: u64) -> Self {
        let mut user_account = AccountData::new(0, system_program::id());
        user_account.is_signer = true;
        let mut swap_account = AccountData::new(SwapInfo::LEN, spl_token_swap::id());
        let (authority_key, nonce) = Pubkey::find_program_address(
            &[&swap_account.key.to_bytes()[..]],
            &spl_token_swap::id(),
        );
        let mut authority_account = create_program_account(authority_key);
        let mut token_program_account = create_program_account(spl_token::id());

        let mut pool_mint_account = create_mint(&authority_account.key);
        let mut pool_token_account =
            create_token_account(&mut pool_mint_account, &user_account.key, 0);
        let mut pool_fee_account =
            create_token_account(&mut pool_mint_account, &user_account.key, 0);
        let mut token_a_mint_account = create_mint(&user_account.key);
        let mut token_a_account = create_token_account(
            &mut token_a_mint_account,
            &authority_account.key,
            token_a_amount,
        );
        let mut token_b_mint_account = create_mint(&user_account.key);
        let mut token_b_account = create_token_account(
            &mut token_b_mint_account,
            &authority_account.key,
            token_b_amount,
        );

        let init_instruction = instruction::initialize(
            &spl_token_swap::id(),
            &spl_token::id(),
            &swap_account.key,
            &authority_account.key,
            &token_a_account.key,
            &token_b_account.key,
            &pool_mint_account.key,
            &pool_fee_account.key,
            &pool_token_account.key,
            nonce,
            swap_curve.clone(),
        )
        .unwrap();

        do_process_instruction(
            init_instruction,
            &[
                swap_account.into_account_info(),
                authority_account.into_account_info(),
                token_a_account.into_account_info(),
                token_b_account.into_account_info(),
                pool_mint_account.into_account_info(),
                pool_fee_account.into_account_info(),
                pool_token_account.into_account_info(),
                token_program_account.into_account_info(),
            ],
        )
        .unwrap();

        Self {
            user_account,
            nonce,
            authority_account,
            swap_curve,
            swap_account,
            pool_mint_account,
            pool_fee_account,
            pool_token_account,
            token_a_account,
            token_a_mint_account,
            token_b_account,
            token_b_mint_account,
            token_program_account,
        }
    }

    pub fn create_pool_account(&mut self) -> AccountData {
        create_token_account(&mut self.pool_mint_account, &self.user_account.key, 0)
    }

    pub fn create_token_a_account(&mut self, amount: u64) -> AccountData {
        create_token_account(
            &mut self.token_a_mint_account,
            &self.user_account.key,
            amount,
        )
    }

    pub fn create_token_b_account(&mut self, amount: u64) -> AccountData {
        create_token_account(
            &mut self.token_b_mint_account,
            &self.user_account.key,
            amount,
        )
    }

    pub fn swap_a_to_b(
        &mut self,
        token_a_account: &mut AccountData,
        token_b_account: &mut AccountData,
        instruction: Swap,
    ) -> ProgramResult {
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_a_account.key,
                &self.authority_account.key,
                &self.user_account.key,
                &[],
                instruction.amount_in,
            )
            .unwrap(),
            &[
                token_a_account.into_account_info(),
                self.authority_account.into_account_info(),
                self.user_account.into_account_info(),
            ],
        )
        .unwrap();
        let swap_instruction = instruction::swap(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &token_a_account.key,
            &self.token_a_account.key,
            &self.token_b_account.key,
            &token_b_account.key,
            &self.pool_mint_account.key,
            &self.pool_fee_account.key,
            Some(&self.pool_token_account.key),
            instruction,
        )
        .unwrap();

        do_process_instruction(
            swap_instruction,
            &[
                self.swap_account.into_account_info(),
                self.authority_account.into_account_info(),
                token_a_account.into_account_info(),
                self.token_a_account.into_account_info(),
                self.token_b_account.into_account_info(),
                token_b_account.into_account_info(),
                self.pool_mint_account.into_account_info(),
                self.pool_fee_account.into_account_info(),
                self.token_program_account.into_account_info(),
                self.pool_token_account.into_account_info(),
            ],
        )
    }

    pub fn swap_b_to_a(
        &mut self,
        token_b_account: &mut AccountData,
        token_a_account: &mut AccountData,
        instruction: Swap,
    ) -> ProgramResult {
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_b_account.key,
                &self.authority_account.key,
                &self.user_account.key,
                &[],
                instruction.amount_in,
            )
            .unwrap(),
            &[
                token_b_account.into_account_info(),
                self.authority_account.into_account_info(),
                self.user_account.into_account_info(),
            ],
        )
        .unwrap();

        let swap_instruction = instruction::swap(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &token_b_account.key,
            &self.token_b_account.key,
            &self.token_a_account.key,
            &token_a_account.key,
            &self.pool_mint_account.key,
            &self.pool_fee_account.key,
            Some(&self.pool_token_account.key),
            instruction,
        )
        .unwrap();

        do_process_instruction(
            swap_instruction,
            &[
                self.swap_account.into_account_info(),
                self.authority_account.into_account_info(),
                token_b_account.into_account_info(),
                self.token_b_account.into_account_info(),
                self.token_a_account.into_account_info(),
                token_a_account.into_account_info(),
                self.pool_mint_account.into_account_info(),
                self.pool_fee_account.into_account_info(),
                self.token_program_account.into_account_info(),
                self.pool_token_account.into_account_info(),
            ],
        )
    }

    pub fn deposit(
        &mut self,
        token_a_account: &mut AccountData,
        token_b_account: &mut AccountData,
        pool_account: &mut AccountData,
        mut instruction: Deposit,
    ) -> ProgramResult {
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_a_account.key,
                &self.authority_account.key,
                &self.user_account.key,
                &[],
                instruction.maximum_token_a_amount,
            )
            .unwrap(),
            &[
                token_a_account.into_account_info(),
                self.authority_account.into_account_info(),
                self.user_account.into_account_info(),
            ],
        )
        .unwrap();

        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_b_account.key,
                &self.authority_account.key,
                &self.user_account.key,
                &[],
                instruction.maximum_token_b_amount,
            )
            .unwrap(),
            &[
                token_b_account.into_account_info(),
                self.authority_account.into_account_info(),
                self.user_account.into_account_info(),
            ],
        )
        .unwrap();

        // special logic: if we only deposit 1 pool token, we can't withdraw it
        // because we incur a withdrawal fee, so we hack it to not be 1
        if instruction.pool_token_amount == 1 {
            instruction.pool_token_amount = 2;
        }

        let deposit_instruction = instruction::deposit(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &token_a_account.key,
            &token_b_account.key,
            &self.token_a_account.key,
            &self.token_b_account.key,
            &self.pool_mint_account.key,
            &pool_account.key,
            instruction,
        )
        .unwrap();

        do_process_instruction(
            deposit_instruction,
            &[
                self.swap_account.into_account_info(),
                self.authority_account.into_account_info(),
                token_a_account.into_account_info(),
                token_b_account.into_account_info(),
                self.token_a_account.into_account_info(),
                self.token_b_account.into_account_info(),
                self.pool_mint_account.into_account_info(),
                pool_account.into_account_info(),
                self.token_program_account.into_account_info(),
            ],
        )
    }

    pub fn withdraw(
        &mut self,
        pool_account: &mut AccountData,
        token_a_account: &mut AccountData,
        token_b_account: &mut AccountData,
        mut instruction: Withdraw,
    ) -> ProgramResult {
        let pool_token_amount = get_token_balance(&pool_account);
        // special logic to avoid withdrawing down to 1 pool token, which
        // eventually causes an error on withdrawing all
        if pool_token_amount.saturating_sub(instruction.pool_token_amount) == 1 {
            instruction.pool_token_amount = pool_token_amount;
        }
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &pool_account.key,
                &self.authority_account.key,
                &self.user_account.key,
                &[],
                instruction.pool_token_amount,
            )
            .unwrap(),
            &[
                pool_account.into_account_info(),
                self.authority_account.into_account_info(),
                self.user_account.into_account_info(),
            ],
        )
        .unwrap();

        let withdraw_instruction = instruction::withdraw(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &self.pool_mint_account.key,
            &self.pool_fee_account.key,
            &pool_account.key,
            &self.token_a_account.key,
            &self.token_b_account.key,
            &token_a_account.key,
            &token_b_account.key,
            instruction,
        )
        .unwrap();

        do_process_instruction(
            withdraw_instruction,
            &[
                self.swap_account.into_account_info(),
                self.authority_account.into_account_info(),
                self.pool_mint_account.into_account_info(),
                pool_account.into_account_info(),
                self.token_a_account.into_account_info(),
                self.token_b_account.into_account_info(),
                token_a_account.into_account_info(),
                token_b_account.into_account_info(),
                self.pool_fee_account.into_account_info(),
                self.token_program_account.into_account_info(),
            ],
        )
    }

    pub fn withdraw_all(
        &mut self,
        mut pool_account: &mut AccountData,
        mut token_a_account: &mut AccountData,
        mut token_b_account: &mut AccountData,
    ) -> ProgramResult {
        let pool_token_amount = get_token_balance(&pool_account);
        if pool_token_amount > 0 {
            let instruction = Withdraw {
                pool_token_amount,
                minimum_token_a_amount: 0,
                minimum_token_b_amount: 0,
            };
            self.withdraw(
                &mut pool_account,
                &mut token_a_account,
                &mut token_b_account,
                instruction,
            )
        } else {
            Ok(())
        }
    }
}
