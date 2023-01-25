use pyth_sdk_solana::state::{
    AccountType, PriceAccount, PriceStatus, ProductAccount, MAGIC, PROD_ACCT_SIZE, PROD_ATTR_SIZE,
    VERSION_2,
};
/// mock oracle prices in tests with this program.
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use std::cell::RefMut;
use switchboard_v2::{AggregatorAccountData, SwitchboardDecimal};

use borsh::{BorshDeserialize, BorshSerialize};
use spl_token::solana_program::{account_info::next_account_info, program_error::ProgramError};
use thiserror::Error;

use super::{load_mut, QUOTE_CURRENCY};

pub mod mock_pyth_program {
    solana_program::declare_id!("SW1TCH7qEPTdLsDHRgPuMQjbQxKdH2aBStViMFnt64f");
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum MockPythInstruction {
    /// Accounts:
    /// 0: PriceAccount (uninitialized)
    /// 1: ProductAccount (uninitialized)
    Init,

    /// Accounts:
    /// 0: PriceAccount
    SetPrice { price: i64, conf: u64, expo: i32 },

    /// Accounts:
    /// 0: AggregatorAccount
    InitSwitchboard,

    /// Accounts:
    /// 0: AggregatorAccount
    SetSwitchboardPrice { price: i64, expo: i32 },
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, instruction_data)
}

pub struct Processor;
impl Processor {
    pub fn process(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = MockPythInstruction::try_from_slice(instruction_data)?;
        let account_info_iter = &mut accounts.iter().peekable();

        match instruction {
            MockPythInstruction::Init => {
                msg!("Mock Pyth: Init");

                let price_account_info = next_account_info(account_info_iter)?;
                let product_account_info = next_account_info(account_info_iter)?;

                // write PriceAccount
                let price_account = PriceAccount {
                    magic: MAGIC,
                    ver: VERSION_2,
                    atype: AccountType::Price as u32,
                    size: 240, // PC_PRICE_T_COMP_OFFSET from pyth_client repo
                    ..PriceAccount::default()
                };

                let mut data = price_account_info.try_borrow_mut_data()?;
                data.copy_from_slice(bytemuck::bytes_of(&price_account));

                // write ProductAccount
                let attr = {
                    let mut attr: Vec<u8> = Vec::new();
                    let quote_currency = b"quote_currency";
                    attr.push(quote_currency.len() as u8);
                    attr.extend(quote_currency);
                    attr.push(QUOTE_CURRENCY.len() as u8);
                    attr.extend(QUOTE_CURRENCY);

                    let mut buf = [0; PROD_ATTR_SIZE];
                    buf[0..attr.len()].copy_from_slice(&attr);

                    buf
                };

                let product_account = ProductAccount {
                    magic: MAGIC,
                    ver: VERSION_2,
                    atype: AccountType::Product as u32,
                    size: PROD_ACCT_SIZE as u32,
                    px_acc: *price_account_info.key,
                    attr,
                };

                let mut data = product_account_info.try_borrow_mut_data()?;
                data.copy_from_slice(bytemuck::bytes_of(&product_account));

                Ok(())
            }
            MockPythInstruction::SetPrice { price, conf, expo } => {
                msg!("Mock Pyth: Set price");
                let price_account_info = next_account_info(account_info_iter)?;
                let data = &mut price_account_info.try_borrow_mut_data()?;
                let mut price_account: &mut PriceAccount = load_mut(data).unwrap();

                price_account.agg.price = price;
                price_account.agg.conf = conf;
                price_account.expo = expo;

                price_account.last_slot = Clock::get()?.slot;
                price_account.agg.pub_slot = Clock::get()?.slot;
                price_account.agg.status = PriceStatus::Trading;

                Ok(())
            }
            MockPythInstruction::InitSwitchboard => {
                msg!("Mock Pyth: Init Switchboard");
                let switchboard_feed = next_account_info(account_info_iter)?;
                let mut data = switchboard_feed.try_borrow_mut_data()?;

                let discriminator = [217, 230, 65, 101, 201, 162, 27, 125];
                data[0..8].copy_from_slice(&discriminator);
                Ok(())
            }
            MockPythInstruction::SetSwitchboardPrice { price, expo } => {
                msg!("Mock Pyth: Set Switchboard price");
                let switchboard_feed = next_account_info(account_info_iter)?;
                let data = switchboard_feed.try_borrow_mut_data()?;

                let mut aggregator_account: RefMut<AggregatorAccountData> =
                    RefMut::map(data, |data| {
                        bytemuck::from_bytes_mut(
                            &mut data[8..std::mem::size_of::<AggregatorAccountData>() + 8],
                        )
                    });

                aggregator_account.min_oracle_results = 1;
                aggregator_account.latest_confirmed_round.num_success = 1;
                aggregator_account.latest_confirmed_round.result = SwitchboardDecimal {
                    mantissa: price as i128,
                    scale: expo as u32,
                };
                aggregator_account.latest_confirmed_round.round_open_slot = Clock::get()?.slot;

                Ok(())
            }
        }
    }
}

#[derive(Error, Debug, Copy, Clone)]
pub enum MockPythError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("The account is not currently owned by the program")]
    IncorrectProgramId,
    #[error("Failed to deserialize")]
    FailedToDeserialize,
}

impl From<MockPythError> for ProgramError {
    fn from(e: MockPythError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub fn init(
    program_id: Pubkey,
    price_account_pubkey: Pubkey,
    product_account_pubkey: Pubkey,
) -> Instruction {
    let data = MockPythInstruction::Init.try_to_vec().unwrap();
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(price_account_pubkey, false),
            AccountMeta::new(product_account_pubkey, false),
        ],
        data,
    }
}

pub fn set_price(
    program_id: Pubkey,
    price_account_pubkey: Pubkey,
    price: i64,
    conf: u64,
    expo: i32,
) -> Instruction {
    let data = MockPythInstruction::SetPrice { price, conf, expo }
        .try_to_vec()
        .unwrap();
    Instruction {
        program_id,
        accounts: vec![AccountMeta::new(price_account_pubkey, false)],
        data,
    }
}

pub fn set_switchboard_price(
    program_id: Pubkey,
    switchboard_feed: Pubkey,
    price: i64,
    expo: i32,
) -> Instruction {
    let data = MockPythInstruction::SetSwitchboardPrice { price, expo }
        .try_to_vec()
        .unwrap();
    Instruction {
        program_id,
        accounts: vec![AccountMeta::new(switchboard_feed, false)],
        data,
    }
}

pub fn init_switchboard(program_id: Pubkey, switchboard_feed: Pubkey) -> Instruction {
    let data = MockPythInstruction::InitSwitchboard.try_to_vec().unwrap();
    Instruction {
        program_id,
        accounts: vec![AccountMeta::new(switchboard_feed, false)],
        data,
    }
}
