use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use borsh::{BorshDeserialize, BorshSerialize};
use spl_token::state::Account;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    error::PerpetualSwapError, instruction::PerpetualSwapInstruction, state::PerpetualSwap,
};

pub struct Processor;
impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Account, PerpetualSwapError> {
        if account_info.owner != token_program_id {
            Err(PerpetualSwapError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Account::unpack(&account_info.data.borrow())
                .map_err(|_| PerpetualSwapError::ExpectedAccount)
        }
    }

    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Mint, PerpetualSwapError> {
        if account_info.owner != token_program_id {
            Err(PerpetualSwapError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Mint::unpack(&account_info.data.borrow())
                .map_err(|_| PerpetualSwapError::ExpectedMint)
        }
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        nonce: u8,
    ) -> Result<Pubkey, PerpetualSwapError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
            .or(Err(PerpetualSwapError::InvalidProgramAddress))
    }

    /// Issue a spl_token `MintTo` instruction.
    pub fn token_mint_to<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(&ix, &[mint, destination, authority, token_program], signers)
    }

    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[source, destination, authority, token_program],
            signers,
        )
    }

    pub fn initialize_account<'a>(
        account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        owner: AccountInfo<'a>,
        rent: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let ix = spl_token::instruction::initialize_account(
            token_program.key,
            account.key,
            mint.key,
            owner.key,
        )?;
        invoke(&ix, &[account, mint, owner, rent, token_program])
    }

    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = PerpetualSwapInstruction::unpack(instruction_data)?;

        match instruction {
            PerpetualSwapInstruction::InitializePerpetualSwap {
                nonce,
                funding_rate,
                minimum_margin,
                liquidation_threshold,
            } => {
                msg!("Instruction: InitializePerpetualSwap");
                Self::process_initialize_perpetual_swap(
                    program_id,
                    nonce,
                    funding_rate,
                    minimum_margin,
                    liquidation_threshold,
                    accounts,
                )
            }
            PerpetualSwapInstruction::InitializeSide { amount_to_deposit } => {
                msg!("Instruction: InitializeSide");
                Self::process_initialize_side(program_id, amount_to_deposit, accounts)
            }
            PerpetualSwapInstruction::DepositToMargin { amount_to_deposit } => {
                msg!("Instruction: DepositToMargin");
                Self::process_deposit_to_margin(program_id, amount_to_deposit, accounts)
            }
            PerpetualSwapInstruction::WithdrawFromMargin { amount_to_withdraw } => {
                msg!("Instruction: WithdrawFromMargin");
                Self::process_withdraw_from_margin(program_id, amount_to_withdraw, accounts)
            }
            PerpetualSwapInstruction::TransferLong { amount } => {
                msg!("Instruction: TransferLong");
                Self::process_transfer_long(program_id, amount, accounts)
            }
            PerpetualSwapInstruction::TransferShort { amount } => {
                msg!("Instruction: TransferShort");
                Self::process_transfer_short(program_id, amount, accounts)
            }
            PerpetualSwapInstruction::TryToLiquidate {} => {
                msg!("Instruction: TryToLiquidate");
                Self::process_try_to_liquidate(program_id, accounts)
            }
            PerpetualSwapInstruction::TransferFunds {} => {
                msg!("Instruction: TransferFunds");
                Self::process_transfer_funds(program_id, accounts)
            }
            PerpetualSwapInstruction::UpdateMarkPrice { price } => {
                msg!("Instruction: UpdateMarkPrice");
                Self::process_update_mark_price(program_id, price, accounts)
            }
            PerpetualSwapInstruction::UpdateIndexPrice { price } => {
                msg!("Instruction: UpdateIndexPrice");
                Self::process_update_index_price(program_id, price, accounts)
            }
        }
    }

    pub fn process_initialize_perpetual_swap(
        program_id: &Pubkey,
        nonce: u8,
        funding_rate: f64,
        minimum_margin: u64,
        liquidation_threshold: f64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let margin_long_info = next_account_info(account_info_iter)?;
        let margin_short_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_program_id = *token_program_info.key;

        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;
        // Check if the perpetual swap is already initialized
        if perpetual_swap.is_initialized() {
            return Err(PerpetualSwapError::AlreadyInUse.into());
        }

        // Check if pool account is rent-exempt
        let rent = &Rent::from_account_info(rent_info)?;
        if !rent.is_exempt(
            perpetual_swap_info.lamports(),
            perpetual_swap_info.data_len(),
        ) {
            return Err(PerpetualSwapError::NotRentExempt.into());
        }

        // Check if the long margin account is already initialized
        let long_margin_account = Account::unpack_unchecked(&margin_long_info.data.borrow())?;
        if long_margin_account.is_initialized() {
            return Err(PerpetualSwapError::AlreadyInUse.into());
        }

        // Check if the short margin account is already initialized
        let short_margin_account = Account::unpack_unchecked(&margin_short_info.data.borrow())?;
        if short_margin_account.is_initialized() {
            return Err(PerpetualSwapError::AlreadyInUse.into());
        }

        let authority_pubkey = Self::authority_id(program_id, perpetual_swap_info.key, nonce)?;

        if *authority_info.key != authority_pubkey {
            return Err(PerpetualSwapError::InvalidAuthorityAccount.into());
        }

        Self::initialize_account(
            perpetual_swap_info.clone(),
            pool_mint_info.clone(),
            authority_info.clone(),
            rent_info.clone(),
            token_program_info.clone(),
        )?;

        Self::initialize_account(
            margin_long_info.clone(),
            pool_mint_info.clone(),
            authority_info.clone(),
            rent_info.clone(),
            token_program_info.clone(),
        )?;

        Self::initialize_account(
            margin_short_info.clone(),
            pool_mint_info.clone(),
            authority_info.clone(),
            rent_info.clone(),
            token_program_info.clone(),
        )?;

        perpetual_swap.is_long_initialized = false;
        perpetual_swap.is_short_initialized = false;
        perpetual_swap.nonce = nonce;
        perpetual_swap.token_program_id = token_program_id;
        perpetual_swap.long_margin_pubkey = *margin_long_info.key;
        perpetual_swap.short_margin_pubkey = *margin_short_info.key;
        perpetual_swap.minimum_margin = minimum_margin;
        perpetual_swap.liquidation_threshold = liquidation_threshold;
        perpetual_swap.funding_rate = funding_rate;
        perpetual_swap
            .serialize(&mut *perpetual_swap_info.data.borrow_mut())
            .map_err(|e| e.into())
    }

    pub fn process_initialize_side(
        program_id: &Pubkey,
        amount_to_deposit: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let margin_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;
        let source_account =
            Self::unpack_token_account(margin_info, &perpetual_swap.token_program_id)?;
        // TODO Add all the data checks
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        let is_long = *margin_info.key == perpetual_swap.long_margin_pubkey;
        let is_short = *margin_info.key == perpetual_swap.short_margin_pubkey;

        if !is_long && !is_short {
            return Err(PerpetualSwapError::InvalidAccountKeys.into());
        }

        if amount_to_deposit < perpetual_swap.minimum_margin {
            return Err(PerpetualSwapError::WouldBeLiquidated.into());
        }

        if source_account.amount < amount_to_deposit {
            return Err(PerpetualSwapError::InsufficientFunds.into());
        }

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            margin_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            amount_to_deposit,
        )?;

        if is_long {
            perpetual_swap.long_account_pubkey = *source_info.key;
            perpetual_swap.is_long_initialized = true;
        } else {
            perpetual_swap.short_account_pubkey = *source_info.key;
            perpetual_swap.is_short_initialized = true;
        }

        // Start the funding rate interval only when both parties have been set
        if perpetual_swap.is_initialized() {
            // This is number of millisecondss ince the epoch
            perpetual_swap.reference_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
        }
        Ok(())
    }

    pub fn process_deposit_to_margin(
        program_id: &Pubkey,
        amount_to_deposit: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let margin_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;
        let source_account =
            Self::unpack_token_account(margin_info, &perpetual_swap.token_program_id)?;
        // TODO Add all the data checks
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        let is_long = *margin_info.key == perpetual_swap.long_margin_pubkey
            && *source_info.key == perpetual_swap.long_account_pubkey;
        let is_short = *margin_info.key == perpetual_swap.short_margin_pubkey
            && *source_info.key == perpetual_swap.short_account_pubkey;

        if !is_long && !is_short {
            return Err(PerpetualSwapError::InvalidAccountKeys.into());
        }

        if source_account.amount < amount_to_deposit {
            return Err(PerpetualSwapError::InsufficientFunds.into());
        }

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            margin_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            amount_to_deposit,
        )?;

        Ok(())
    }

    pub fn process_withdraw_from_margin(
        program_id: &Pubkey,
        amount_to_withdraw: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let margin_info = next_account_info(account_info_iter)?;
        let dest_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;
        let source_account =
            Self::unpack_token_account(margin_info, &perpetual_swap.token_program_id)?;

        // TODO add all the data checks
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        let is_long = *margin_info.key == perpetual_swap.long_margin_pubkey
            && *dest_info.key == perpetual_swap.long_account_pubkey;
        let is_short = *margin_info.key == perpetual_swap.short_margin_pubkey
            && *dest_info.key == perpetual_swap.short_account_pubkey;

        if !is_long && !is_short {
            return Err(PerpetualSwapError::InvalidAccountKeys.into());
        }

        if source_account.amount - amount_to_withdraw < perpetual_swap.minimum_margin {
            return Err(PerpetualSwapError::WouldBeLiquidated.into());
        }

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            margin_info.clone(),
            dest_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            amount_to_withdraw,
        )?;

        Ok(())
    }

    pub fn process_transfer_long(
        program_id: &Pubkey,
        margin_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let long_margin_info = next_account_info(account_info_iter)?;
        let long_account_info = next_account_info(account_info_iter)?;
        let new_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;
        let long_margin =
            Self::unpack_token_account(long_margin_info, &perpetual_swap.token_program_id)?;
        let long_account =
            Self::unpack_token_account(long_account_info, &perpetual_swap.token_program_id)?;
        let new_account =
            Self::unpack_token_account(new_account_info, &perpetual_swap.token_program_id)?;

        // TODO add more checks
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }
        if perpetual_swap.long_margin_pubkey != *long_margin_info.key
            || perpetual_swap.long_account_pubkey != *long_account_info.key
        {
            return Err(PerpetualSwapError::InvalidAccountKeys.into());
        }
        if long_account.mint != new_account.mint {
            return Err(PerpetualSwapError::InvalidMints.into());
        }

        if margin_amount < perpetual_swap.minimum_margin {
            return Err(PerpetualSwapError::InsufficientMargin.into());
        }

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            long_margin_info.clone(),
            long_account_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            long_margin.amount,
        )?;

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            new_account_info.clone(),
            long_margin_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            margin_amount,
        )?;

        perpetual_swap.long_account_pubkey = *new_account_info.key;

        Ok(())
    }

    pub fn process_transfer_short(
        program_id: &Pubkey,
        margin_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let short_margin_info = next_account_info(account_info_iter)?;
        let short_account_info = next_account_info(account_info_iter)?;
        let new_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;
        let short_margin_account =
            Self::unpack_token_account(short_margin_info, &perpetual_swap.token_program_id)?;

        // TODO add all the checks
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }
        if perpetual_swap.short_margin_pubkey != *short_margin_info.key
            || perpetual_swap.short_account_pubkey != *short_account_info.key
        {
            return Err(PerpetualSwapError::InvalidAccountKeys.into());
        }

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            short_margin_info.clone(),
            short_account_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            short_margin_account.amount,
        )?;

        Self::token_transfer(
            perpetual_swap_info.key,
            token_program_info.clone(),
            new_account_info.clone(),
            short_margin_info.clone(),
            user_transfer_authority_info.clone(),
            perpetual_swap.nonce,
            margin_amount,
        )?;

        perpetual_swap.short_account_pubkey = *new_account_info.key;
        Ok(())
    }

    pub fn process_transfer_funds(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let long_margin_info = next_account_info(account_info_iter)?;
        let short_margin_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;

        // TODO add more checks
        if !perpetual_swap.is_initialized() {
            return Err(PerpetualSwapError::AccountNotInitialized.into());
        }

        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH);
        // This is number of milliseconds since the epoch
        let transfer_time = now.unwrap().as_millis();
        if perpetual_swap.reference_time > transfer_time {
            return Err(PerpetualSwapError::InvalidTransferTime.into());
        }
        let time_since_last_transfer = transfer_time - perpetual_swap.reference_time;
        // funding_rate = base_funding rate * (amount of time since last transfer) / (# of ms in 1 day)
        let funding_interval = time_since_last_transfer as f64 / (24. * 60. * 60. * 1000.) as f64;
        let funding_rate = perpetual_swap.funding_rate * funding_interval;

        // TODO check for liquidation
        if perpetual_swap.mark_price - perpetual_swap.index_price > 0.0 {
            // This is subject to some rounding error
            let funds_to_transfer =
                ((perpetual_swap.mark_price - perpetual_swap.index_price) * funding_rate) as u64;
            Self::token_transfer(
                perpetual_swap_info.key,
                token_program_info.clone(),
                long_margin_info.clone(),
                short_margin_info.clone(),
                user_transfer_authority_info.clone(),
                perpetual_swap.nonce,
                funds_to_transfer,
            )?;
        } else {
            // This is subject to some rounding error
            let funds_to_transfer =
                ((perpetual_swap.index_price - perpetual_swap.mark_price) * funding_rate) as u64;
            Self::token_transfer(
                perpetual_swap_info.key,
                token_program_info.clone(),
                short_margin_info.clone(),
                long_margin_info.clone(),
                user_transfer_authority_info.clone(),
                perpetual_swap.nonce,
                funds_to_transfer,
            )?;
        }
        perpetual_swap.reference_time = transfer_time;
        Ok(())
    }

    pub fn process_try_to_liquidate(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let long_margin_info = next_account_info(account_info_iter)?;
        let long_account_info = next_account_info(account_info_iter)?;
        let short_margin_info = next_account_info(account_info_iter)?;
        let short_account_info = next_account_info(account_info_iter)?;
        let insurance_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;

        if !perpetual_swap.is_initialized() {
            return Err(PerpetualSwapError::AccountNotInitialized.into());
        }
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        let long_margin =
            Self::unpack_token_account(long_margin_info, &perpetual_swap.token_program_id)?;
        let long_account =
            Self::unpack_token_account(long_account_info, &perpetual_swap.token_program_id)?;
        let short_margin =
            Self::unpack_token_account(short_margin_info, &perpetual_swap.token_program_id)?;
        let short_account =
            Self::unpack_token_account(short_account_info, &perpetual_swap.token_program_id)?;

        let mut needs_liquidation = false;
        let mut liquidated_margin_account_info = long_margin_info.clone();
        let mut liquidated_user_account_info = long_account_info.clone();
        let mut receiver_margin_account_info = short_margin_info.clone();
        let mut receiver_user_account_info = short_account_info.clone();
        let mut amount_owed = 0;
        let mut amount_paid = 0;
        let mut liquidation_fee = 0;
        let mut receiver_margin_account_total = 0;
        let mut returned_amount = 0;
        if perpetual_swap.mark_price > perpetual_swap.index_price {
            if long_account.amount <= perpetual_swap.minimum_margin {
                needs_liquidation = true;
                amount_owed = (perpetual_swap.mark_price - perpetual_swap.index_price) as u64;
                amount_paid = std::cmp::min(amount_owed, long_margin.amount);
                liquidation_fee = ((long_margin.amount - amount_paid) as f64
                    * perpetual_swap.liquidation_threshold)
                    as u64;
                if long_account.amount > amount_paid + liquidation_fee {
                    returned_amount = long_account.amount - amount_paid - liquidation_fee;
                }
            }
        } else if short_account.amount <= perpetual_swap.minimum_margin {
            needs_liquidation = true;
            liquidated_margin_account_info = short_margin_info.clone();
            liquidated_user_account_info = short_account_info.clone();
            receiver_margin_account_info = long_margin_info.clone();
            receiver_user_account_info = long_account_info.clone();
            receiver_margin_account_total = long_account.amount;
            amount_owed = (perpetual_swap.index_price - perpetual_swap.mark_price) as u64;
            amount_paid = std::cmp::min(amount_owed, long_margin.amount);
            liquidation_fee = ((short_margin.amount - amount_paid) as f64
                * perpetual_swap.liquidation_threshold) as u64;
            if short_account.amount > amount_paid + liquidation_fee {
                returned_amount = short_account.amount - amount_paid - liquidation_fee;
            }
        }

        if needs_liquidation {
            // Clear the receiver's margin account
            Self::token_transfer(
                perpetual_swap_info.key,
                token_program_info.clone(),
                receiver_margin_account_info.clone(),
                receiver_user_account_info.clone(),
                user_transfer_authority_info.clone(),
                perpetual_swap.nonce,
                receiver_margin_account_total,
            )?;
            // Liquidate the user who is past margin
            Self::token_transfer(
                perpetual_swap_info.key,
                token_program_info.clone(),
                liquidated_margin_account_info.clone(),
                receiver_user_account_info.clone(),
                user_transfer_authority_info.clone(),
                perpetual_swap.nonce,
                amount_paid,
            )?;
            // Pay a liquidation fee to the insurance account
            if liquidation_fee > 0 {
                Self::token_transfer(
                    perpetual_swap_info.key,
                    token_program_info.clone(),
                    liquidated_margin_account_info.clone(),
                    insurance_account_info.clone(),
                    user_transfer_authority_info.clone(),
                    perpetual_swap.nonce,
                    liquidation_fee,
                )?;
            }
            // Return any remaining funds to the liquidated user
            if returned_amount > 0 {
                Self::token_transfer(
                    perpetual_swap_info.key,
                    token_program_info.clone(),
                    liquidated_margin_account_info.clone(),
                    liquidated_user_account_info.clone(),
                    user_transfer_authority_info.clone(),
                    perpetual_swap.nonce,
                    returned_amount,
                )?;
            }
            // If the liquidated user cannot cover his/her losses, we pull from the insurance account
            if amount_paid < amount_owed {
                let insurance = amount_owed - amount_paid;
                Self::token_transfer(
                    perpetual_swap_info.key,
                    token_program_info.clone(),
                    insurance_account_info.clone(),
                    receiver_user_account_info.clone(),
                    user_transfer_authority_info.clone(),
                    perpetual_swap.nonce,
                    insurance,
                )?;
            }
        }
        Ok(())
    }

    pub fn process_update_mark_price(
        program_id: &Pubkey,
        price: f64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;

        if !perpetual_swap.is_initialized() {
            return Err(PerpetualSwapError::AccountNotInitialized.into());
        }
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        perpetual_swap.mark_price = price;
        Ok(())
    }

    pub fn process_update_index_price(
        program_id: &Pubkey,
        price: f64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let perpetual_swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let mut perpetual_swap = PerpetualSwap::try_from_slice(&perpetual_swap_info.data.borrow())?;

        if !perpetual_swap.is_initialized() {
            return Err(PerpetualSwapError::AccountNotInitialized.into());
        }
        if perpetual_swap_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        if *authority_info.key
            != Self::authority_id(program_id, perpetual_swap_info.key, perpetual_swap.nonce)?
        {
            return Err(PerpetualSwapError::InvalidProgramAddress.into());
        }
        if *token_program_info.key != perpetual_swap.token_program_id {
            return Err(PerpetualSwapError::IncorrectTokenProgramId.into());
        }

        perpetual_swap.index_price = price;
        Ok(())
    }
}
