//! User info account management.

use {
    solana_farm_sdk::{
        error::FarmError,
        fund::Fund,
        refdb,
        refdb::{RefDB, Reference, ReferenceType, StorageType},
        string::{str_to_as64, ArrayString64},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::cell::RefMut,
};

pub struct UserInfo<'a, 'b> {
    pub key: &'a Pubkey,
    pub data: RefMut<'a, &'b mut [u8]>,
}

impl<'a, 'b> UserInfo<'a, 'b> {
    pub const LEN: usize = StorageType::get_storage_size_for_records(ReferenceType::U64, 2);
    pub const VIRTUAL_TOKENS_BALANCE_INDEX: usize = 0;
    pub const USER_BUMP_INDEX: usize = 1;

    pub fn new(account: &'a AccountInfo<'b>) -> Self {
        Self {
            key: account.key,
            data: account.data.borrow_mut(),
        }
    }

    pub fn init(&mut self, refdb_name: &ArrayString64, user_bump: u8) -> ProgramResult {
        if RefDB::is_initialized(&self.data) {
            return Ok(());
        }
        RefDB::init(&mut self.data, refdb_name, ReferenceType::U64)?;

        self.init_refdb_field(
            UserInfo::VIRTUAL_TOKENS_BALANCE_INDEX,
            "VirtualTokensBalance",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            UserInfo::USER_BUMP_INDEX,
            "UserBump",
            Reference::U64 {
                data: user_bump as u64,
            },
        )
    }

    pub fn set_virtual_tokens_balance(&mut self, virtual_tokens_balance: u64) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            UserInfo::VIRTUAL_TOKENS_BALANCE_INDEX,
            &Reference::U64 {
                data: virtual_tokens_balance,
            },
        )
        .map(|_| ())
    }

    pub fn get_virtual_tokens_balance(&self) -> Result<u64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, UserInfo::VIRTUAL_TOKENS_BALANCE_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data);
            }
        }
        Err(FarmError::InvalidRefdbRecord.into())
    }

    pub fn get_user_bump(&self) -> Result<u8, ProgramError> {
        if let Some(user_bump_rec) = RefDB::read_at(&self.data, UserInfo::USER_BUMP_INDEX)? {
            if let Reference::U64 { data } = user_bump_rec.reference {
                return Ok(data as u8);
            }
        }
        Err(ProgramError::UninitializedAccount)
    }

    pub fn validate_account(
        fund: &Fund,
        user_info_account: &'a AccountInfo<'b>,
        user_account: &Pubkey,
    ) -> bool {
        if let Ok(refdb) = user_info_account.try_borrow_data() {
            if let Ok(Some(user_bump_rec)) = RefDB::read_at(&refdb, UserInfo::USER_BUMP_INDEX) {
                if let Reference::U64 { data } = user_bump_rec.reference {
                    if let Ok(key) = Pubkey::create_program_address(
                        &[
                            b"user_info_account",
                            user_account.as_ref(),
                            fund.name.as_bytes(),
                            &[data as u8],
                        ],
                        &fund.fund_program_id,
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
