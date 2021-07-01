//! Helpers for working with swaps in a fuzzing environment

use crate::native_account_data::NativeAccountData;
use crate::native_processor::do_process_instruction;
use crate::native_token;

use spl_token_swap::{
    curve::{base::SwapCurve, fees::Fees},
    instruction::{
        self, DepositAllTokenTypes, DepositSingleTokenTypeExactAmountIn, Swap,
        WithdrawAllTokenTypes, WithdrawSingleTokenTypeExactAmountOut,
    },
    state::SwapVersion,
};

use spl_token::instruction::approve;

use solana_program::{bpf_loader, entrypoint::ProgramResult, pubkey::Pubkey, system_program};

pub struct NativeTokenSwap {
    pub user_account: NativeAccountData,
    pub nonce: u8,
    pub authority_account: NativeAccountData,
    pub fees: Fees,
    pub swap_curve: SwapCurve,
    pub swap_account: NativeAccountData,
    pub pool_mint_account: NativeAccountData,
    pub pool_fee_account: NativeAccountData,
    pub pool_token_account: NativeAccountData,
    pub token_a_account: NativeAccountData,
    pub token_a_mint_account: NativeAccountData,
    pub token_b_account: NativeAccountData,
    pub token_b_mint_account: NativeAccountData,
    pub token_program_account: NativeAccountData,
}

pub fn create_program_account(program_id: Pubkey) -> NativeAccountData {
    let mut account_data = NativeAccountData::new(0, bpf_loader::id());
    account_data.key = program_id;
    account_data
}

impl NativeTokenSwap {
    pub fn new(
        fees: Fees,
        swap_curve: SwapCurve,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Self {
        let mut user_account = NativeAccountData::new(0, system_program::id());
        user_account.is_signer = true;
        let mut swap_account =
            NativeAccountData::new(SwapVersion::LATEST_LEN, spl_token_swap::id());
        let (authority_key, nonce) = Pubkey::find_program_address(
            &[&swap_account.key.to_bytes()[..]],
            &spl_token_swap::id(),
        );
        let mut authority_account = create_program_account(authority_key);
        let mut token_program_account = create_program_account(spl_token::id());

        let mut pool_mint_account = native_token::create_mint(&authority_account.key);
        let mut pool_token_account =
            native_token::create_token_account(&mut pool_mint_account, &user_account.key, 0);
        let mut pool_fee_account =
            native_token::create_token_account(&mut pool_mint_account, &user_account.key, 0);
        let mut token_a_mint_account = native_token::create_mint(&user_account.key);
        let mut token_a_account = native_token::create_token_account(
            &mut token_a_mint_account,
            &authority_account.key,
            token_a_amount,
        );
        let mut token_b_mint_account = native_token::create_mint(&user_account.key);
        let mut token_b_account = native_token::create_token_account(
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
            fees.clone(),
            swap_curve.clone(),
        )
        .unwrap();

        do_process_instruction(
            init_instruction,
            &[
                swap_account.as_account_info(),
                authority_account.as_account_info(),
                token_a_account.as_account_info(),
                token_b_account.as_account_info(),
                pool_mint_account.as_account_info(),
                pool_fee_account.as_account_info(),
                pool_token_account.as_account_info(),
                token_program_account.as_account_info(),
            ],
        )
        .unwrap();

        Self {
            user_account,
            nonce,
            authority_account,
            fees,
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

    pub fn create_pool_account(&mut self) -> NativeAccountData {
        native_token::create_token_account(&mut self.pool_mint_account, &self.user_account.key, 0)
    }

    pub fn create_token_a_account(&mut self, amount: u64) -> NativeAccountData {
        native_token::create_token_account(
            &mut self.token_a_mint_account,
            &self.user_account.key,
            amount,
        )
    }

    pub fn create_token_b_account(&mut self, amount: u64) -> NativeAccountData {
        native_token::create_token_account(
            &mut self.token_b_mint_account,
            &self.user_account.key,
            amount,
        )
    }

    pub fn swap_a_to_b(
        &mut self,
        token_a_account: &mut NativeAccountData,
        token_b_account: &mut NativeAccountData,
        instruction: Swap,
    ) -> ProgramResult {
        let mut user_transfer_account = NativeAccountData::new(0, system_program::id());
        user_transfer_account.is_signer = true;
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_a_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.amount_in,
            )
            .unwrap(),
            &[
                token_a_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();
        let swap_instruction = instruction::swap(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &user_transfer_account.key,
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
                self.swap_account.as_account_info(),
                self.authority_account.as_account_info(),
                user_transfer_account.as_account_info(),
                token_a_account.as_account_info(),
                self.token_a_account.as_account_info(),
                self.token_b_account.as_account_info(),
                token_b_account.as_account_info(),
                self.pool_mint_account.as_account_info(),
                self.pool_fee_account.as_account_info(),
                self.token_program_account.as_account_info(),
                self.pool_token_account.as_account_info(),
            ],
        )
    }

    pub fn swap_b_to_a(
        &mut self,
        token_b_account: &mut NativeAccountData,
        token_a_account: &mut NativeAccountData,
        instruction: Swap,
    ) -> ProgramResult {
        let mut user_transfer_account = NativeAccountData::new(0, system_program::id());
        user_transfer_account.is_signer = true;
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_b_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.amount_in,
            )
            .unwrap(),
            &[
                token_b_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();

        let swap_instruction = instruction::swap(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &user_transfer_account.key,
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
                self.swap_account.as_account_info(),
                self.authority_account.as_account_info(),
                user_transfer_account.as_account_info(),
                token_b_account.as_account_info(),
                self.token_b_account.as_account_info(),
                self.token_a_account.as_account_info(),
                token_a_account.as_account_info(),
                self.pool_mint_account.as_account_info(),
                self.pool_fee_account.as_account_info(),
                self.token_program_account.as_account_info(),
                self.pool_token_account.as_account_info(),
            ],
        )
    }

    pub fn deposit_all_token_types(
        &mut self,
        token_a_account: &mut NativeAccountData,
        token_b_account: &mut NativeAccountData,
        pool_account: &mut NativeAccountData,
        mut instruction: DepositAllTokenTypes,
    ) -> ProgramResult {
        let mut user_transfer_account = NativeAccountData::new(0, system_program::id());
        user_transfer_account.is_signer = true;
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_a_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.maximum_token_a_amount,
            )
            .unwrap(),
            &[
                token_a_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();

        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &token_b_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.maximum_token_b_amount,
            )
            .unwrap(),
            &[
                token_b_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();

        // special logic: if we only deposit 1 pool token, we can't withdraw it
        // because we incur a withdrawal fee, so we hack it to not be 1
        if instruction.pool_token_amount == 1 {
            instruction.pool_token_amount = 2;
        }

        let deposit_instruction = instruction::deposit_all_token_types(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &user_transfer_account.key,
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
                self.swap_account.as_account_info(),
                self.authority_account.as_account_info(),
                user_transfer_account.as_account_info(),
                token_a_account.as_account_info(),
                token_b_account.as_account_info(),
                self.token_a_account.as_account_info(),
                self.token_b_account.as_account_info(),
                self.pool_mint_account.as_account_info(),
                pool_account.as_account_info(),
                self.token_program_account.as_account_info(),
            ],
        )
    }

    pub fn withdraw_all_token_types(
        &mut self,
        pool_account: &mut NativeAccountData,
        token_a_account: &mut NativeAccountData,
        token_b_account: &mut NativeAccountData,
        mut instruction: WithdrawAllTokenTypes,
    ) -> ProgramResult {
        let mut user_transfer_account = NativeAccountData::new(0, system_program::id());
        user_transfer_account.is_signer = true;
        let pool_token_amount = native_token::get_token_balance(pool_account);
        // special logic to avoid withdrawing down to 1 pool token, which
        // eventually causes an error on withdrawing all
        if pool_token_amount.saturating_sub(instruction.pool_token_amount) == 1 {
            instruction.pool_token_amount = pool_token_amount;
        }
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &pool_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.pool_token_amount,
            )
            .unwrap(),
            &[
                pool_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();

        let withdraw_instruction = instruction::withdraw_all_token_types(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &user_transfer_account.key,
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
                self.swap_account.as_account_info(),
                self.authority_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.pool_mint_account.as_account_info(),
                pool_account.as_account_info(),
                self.token_a_account.as_account_info(),
                self.token_b_account.as_account_info(),
                token_a_account.as_account_info(),
                token_b_account.as_account_info(),
                self.pool_fee_account.as_account_info(),
                self.token_program_account.as_account_info(),
            ],
        )
    }

    pub fn deposit_single_token_type_exact_amount_in(
        &mut self,
        source_token_account: &mut NativeAccountData,
        pool_account: &mut NativeAccountData,
        mut instruction: DepositSingleTokenTypeExactAmountIn,
    ) -> ProgramResult {
        let mut user_transfer_account = NativeAccountData::new(0, system_program::id());
        user_transfer_account.is_signer = true;
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &source_token_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.source_token_amount,
            )
            .unwrap(),
            &[
                source_token_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();

        // special logic: if we only deposit 1 pool token, we can't withdraw it
        // because we incur a withdrawal fee, so we hack it to not be 1
        if instruction.minimum_pool_token_amount < 2 {
            instruction.minimum_pool_token_amount = 2;
        }

        let deposit_instruction = instruction::deposit_single_token_type_exact_amount_in(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &user_transfer_account.key,
            &source_token_account.key,
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
                self.swap_account.as_account_info(),
                self.authority_account.as_account_info(),
                user_transfer_account.as_account_info(),
                source_token_account.as_account_info(),
                self.token_a_account.as_account_info(),
                self.token_b_account.as_account_info(),
                self.pool_mint_account.as_account_info(),
                pool_account.as_account_info(),
                self.token_program_account.as_account_info(),
            ],
        )
    }

    pub fn withdraw_single_token_type_exact_amount_out(
        &mut self,
        pool_account: &mut NativeAccountData,
        destination_token_account: &mut NativeAccountData,
        mut instruction: WithdrawSingleTokenTypeExactAmountOut,
    ) -> ProgramResult {
        let mut user_transfer_account = NativeAccountData::new(0, system_program::id());
        user_transfer_account.is_signer = true;
        let pool_token_amount = native_token::get_token_balance(pool_account);
        // special logic to avoid withdrawing down to 1 pool token, which
        // eventually causes an error on withdrawing all
        if pool_token_amount.saturating_sub(instruction.maximum_pool_token_amount) == 1 {
            instruction.maximum_pool_token_amount = pool_token_amount;
        }
        do_process_instruction(
            approve(
                &self.token_program_account.key,
                &pool_account.key,
                &user_transfer_account.key,
                &self.user_account.key,
                &[],
                instruction.maximum_pool_token_amount,
            )
            .unwrap(),
            &[
                pool_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.user_account.as_account_info(),
            ],
        )
        .unwrap();

        let withdraw_instruction = instruction::withdraw_single_token_type_exact_amount_out(
            &spl_token_swap::id(),
            &spl_token::id(),
            &self.swap_account.key,
            &self.authority_account.key,
            &user_transfer_account.key,
            &self.pool_mint_account.key,
            &self.pool_fee_account.key,
            &pool_account.key,
            &self.token_a_account.key,
            &self.token_b_account.key,
            &destination_token_account.key,
            instruction,
        )
        .unwrap();

        do_process_instruction(
            withdraw_instruction,
            &[
                self.swap_account.as_account_info(),
                self.authority_account.as_account_info(),
                user_transfer_account.as_account_info(),
                self.pool_mint_account.as_account_info(),
                pool_account.as_account_info(),
                self.token_a_account.as_account_info(),
                self.token_b_account.as_account_info(),
                destination_token_account.as_account_info(),
                self.pool_fee_account.as_account_info(),
                self.token_program_account.as_account_info(),
            ],
        )
    }

    pub fn withdraw_all(
        &mut self,
        mut pool_account: &mut NativeAccountData,
        mut token_a_account: &mut NativeAccountData,
        mut token_b_account: &mut NativeAccountData,
    ) -> ProgramResult {
        let pool_token_amount = native_token::get_token_balance(pool_account);
        if pool_token_amount > 0 {
            let instruction = WithdrawAllTokenTypes {
                pool_token_amount,
                minimum_token_a_amount: 0,
                minimum_token_b_amount: 0,
            };
            self.withdraw_all_token_types(
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
