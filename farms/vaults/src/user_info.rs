//! User info account management.

use {
    crate::clock,
    solana_farm_sdk::{
        math::checked_add,
        refdb,
        refdb::{RefDB, Reference, ReferenceType},
        string::{str_to_as64, ArrayString64},
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, clock::UnixTimestamp, entrypoint::ProgramResult,
        program_error::ProgramError, pubkey::Pubkey,
    },
    std::cell::RefMut,
};

pub struct UserInfo<'a, 'b> {
    pub key: &'a Pubkey,
    pub data: RefMut<'a, &'b mut [u8]>,
}

impl<'a, 'b> UserInfo<'a, 'b> {
    pub const LEN: usize = 681; //StorageType::get_storage_size_for_records(ReferenceType::U64, 8);
    pub const LAST_DEPOSIT_INDEX: usize = 0;
    pub const LAST_WITHDRAWAL_INDEX: usize = 1;
    pub const TOKEN_A_ADDED_INDEX: usize = 2;
    pub const TOKEN_A_REMOVED_INDEX: usize = 3;
    pub const TOKEN_B_ADDED_INDEX: usize = 4;
    pub const TOKEN_B_REMOVED_INDEX: usize = 5;
    pub const LP_TOKENS_DEBT: usize = 6;
    pub const USER_BUMP: usize = 7;

    pub fn new(account: &'a AccountInfo<'b>) -> Self {
        Self {
            key: account.key,
            data: account.data.borrow_mut(),
        }
    }

    pub fn init(&mut self, refdb_name: &ArrayString64, user_bump: u8) -> ProgramResult {
        RefDB::init(&mut self.data, refdb_name, ReferenceType::U64)?;

        self.init_refdb_field(
            UserInfo::LAST_DEPOSIT_INDEX,
            "LastDeposit",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::LAST_WITHDRAWAL_INDEX,
            "LastWithdrawal",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::TOKEN_A_ADDED_INDEX,
            "TokenAAdded",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::TOKEN_A_REMOVED_INDEX,
            "TokenARemoved",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::TOKEN_B_ADDED_INDEX,
            "TokenBAdded",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::TOKEN_B_REMOVED_INDEX,
            "TokenBRemoved",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::LP_TOKENS_DEBT,
            "LpTokensDebt",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::USER_BUMP,
            "UserBump",
            Reference::U64 {
                data: user_bump as u64,
            },
        )
    }

    pub fn update_deposit_time(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            UserInfo::LAST_DEPOSIT_INDEX,
            &Reference::U64 {
                data: clock::get_time_as_u64()?,
            },
        )
        .map(|_| ())
    }

    pub fn update_withdrawal_time(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            UserInfo::LAST_WITHDRAWAL_INDEX,
            &Reference::U64 {
                data: clock::get_time_as_u64()?,
            },
        )
        .map(|_| ())
    }

    pub fn add_liquidity(&mut self, token_a_added: u64, token_b_added: u64) -> ProgramResult {
        if token_a_added > 0 {
            let mut token_a_balance = token_a_added;
            if let Some(token_a_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_A_ADDED_INDEX)? {
                if let Reference::U64 { data } = token_a_rec.reference {
                    token_a_balance = token_a_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                UserInfo::TOKEN_A_ADDED_INDEX,
                &Reference::U64 {
                    data: token_a_balance,
                },
            )?;
        }
        if token_b_added > 0 {
            let mut token_b_balance = token_b_added;
            if let Some(token_b_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_B_ADDED_INDEX)? {
                if let Reference::U64 { data } = token_b_rec.reference {
                    token_b_balance = token_b_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                UserInfo::TOKEN_B_ADDED_INDEX,
                &Reference::U64 {
                    data: token_b_balance,
                },
            )?;
        }
        if token_a_added > 0 || token_b_added > 0 {
            self.update_deposit_time()?;
        }
        Ok(())
    }

    pub fn remove_liquidity(
        &mut self,
        token_a_removed: u64,
        token_b_removed: u64,
    ) -> ProgramResult {
        if token_a_removed > 0 {
            let mut token_a_balance = token_a_removed;
            if let Some(token_a_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_A_REMOVED_INDEX)?
            {
                if let Reference::U64 { data } = token_a_rec.reference {
                    token_a_balance = token_a_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                UserInfo::TOKEN_A_REMOVED_INDEX,
                &Reference::U64 {
                    data: token_a_balance,
                },
            )?;
        }
        if token_b_removed > 0 {
            let mut token_b_balance = token_b_removed;
            if let Some(token_b_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_B_REMOVED_INDEX)?
            {
                if let Reference::U64 { data } = token_b_rec.reference {
                    token_b_balance = token_b_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                UserInfo::TOKEN_B_REMOVED_INDEX,
                &Reference::U64 {
                    data: token_b_balance,
                },
            )?;
        }
        if token_a_removed > 0 || token_b_removed > 0 {
            self.update_withdrawal_time()?;
        }
        Ok(())
    }

    pub fn add_lp_tokens_debt(&mut self, token_added: u64) -> ProgramResult {
        let mut token_debt_total = token_added;
        if let Some(token_debt_rec) = RefDB::read_at(&self.data, UserInfo::LP_TOKENS_DEBT)? {
            if let Reference::U64 { data } = token_debt_rec.reference {
                token_debt_total = checked_add(token_debt_total, data)?;
            }
        }
        RefDB::update_at(
            &mut self.data,
            UserInfo::LP_TOKENS_DEBT,
            &Reference::U64 {
                data: token_debt_total,
            },
        )?;
        Ok(())
    }

    pub fn remove_lp_tokens_debt(&mut self, token_removed: u64) -> ProgramResult {
        let mut token_debt_total = 0;
        if let Some(token_debt_rec) = RefDB::read_at(&self.data, UserInfo::LP_TOKENS_DEBT)? {
            if let Reference::U64 { data } = token_debt_rec.reference {
                token_debt_total = data;
            }
        }
        // safe to use unchecked sub
        if token_debt_total <= token_removed {
            token_debt_total = 0;
        } else {
            token_debt_total -= token_removed;
        }
        RefDB::update_at(
            &mut self.data,
            UserInfo::LP_TOKENS_DEBT,
            &Reference::U64 {
                data: token_debt_total,
            },
        )?;
        Ok(())
    }

    pub fn get_lp_tokens_debt(&self) -> Result<u64, ProgramError> {
        if let Some(token_debt_rec) = RefDB::read_at(&self.data, UserInfo::LP_TOKENS_DEBT)? {
            if let Reference::U64 { data } = token_debt_rec.reference {
                return Ok(data);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_deposit_time(&self) -> Result<UnixTimestamp, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, UserInfo::LAST_DEPOSIT_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data as UnixTimestamp);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_withdrawal_time(&self) -> Result<UnixTimestamp, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, UserInfo::LAST_WITHDRAWAL_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data as UnixTimestamp);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_token_a_added(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_A_ADDED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_token_b_added(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_B_ADDED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_token_a_removed(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_A_REMOVED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_token_b_removed(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, UserInfo::TOKEN_B_REMOVED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn get_user_bump(&self) -> Result<u8, ProgramError> {
        if let Some(user_bump_rec) = RefDB::read_at(&self.data, UserInfo::USER_BUMP)? {
            if let Reference::U64 { data } = user_bump_rec.reference {
                return Ok(data as u8);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn validate_account(
        vault: &Vault,
        user_info_account: &'a AccountInfo<'b>,
        user_account: &Pubkey,
    ) -> bool {
        if let Ok(refdb) = user_info_account.try_borrow_data() {
            if let Ok(Some(user_bump_rec)) = RefDB::read_at(&refdb, UserInfo::USER_BUMP) {
                if let Reference::U64 { data } = user_bump_rec.reference {
                    if let Ok(key) = Pubkey::create_program_address(
                        &[
                            b"user_info_account",
                            &user_account.to_bytes()[..],
                            vault.name.as_bytes(),
                            &[data as u8],
                        ],
                        &vault.vault_program_id,
                    ) {
                        if user_info_account.key == &key {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    // private helpers
    fn init_refdb_field(
        &mut self,
        index: usize,
        field_name: &str,
        reference: Reference,
    ) -> ProgramResult {
        RefDB::write(
            &mut self.data,
            &refdb::Record {
                index: Some(index as u32),
                counter: 0,
                tag: 0,
                name: str_to_as64(field_name)?,
                reference,
            },
        )
        .map(|_| ())
    }
}
