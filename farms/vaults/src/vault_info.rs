//! Vault info account management.

use {
    crate::{clock, traits::VaultParams},
    solana_farm_sdk::{
        refdb,
        refdb::{RefDB, Reference, ReferenceType},
        string::{str_to_as64, ArrayString64},
    },
    solana_program::{
        account_info::AccountInfo, clock::UnixTimestamp, entrypoint::ProgramResult,
        program_error::ProgramError, pubkey::Pubkey,
    },
    std::cell::RefMut,
};

pub struct VaultInfo<'a, 'b> {
    pub key: &'a Pubkey,
    pub data: RefMut<'a, &'b mut [u8]>,
}

impl<'a, 'b> VaultInfo<'a, 'b> {
    pub const LEN: usize = 1061; //StorageType::get_storage_size_for_records(ReferenceType::U64, 13);
    pub const CRANK_TIME_INDEX: usize = 0;
    pub const CRANK_STEP_INDEX: usize = 1;
    pub const TOKEN_A_ADDED_INDEX: usize = 2;
    pub const TOKEN_A_REMOVED_INDEX: usize = 3;
    pub const TOKEN_B_ADDED_INDEX: usize = 4;
    pub const TOKEN_B_REMOVED_INDEX: usize = 5;
    pub const TOKEN_A_REWARDS_INDEX: usize = 6;
    pub const TOKEN_B_REWARDS_INDEX: usize = 7;
    pub const DEPOSIT_ALLOWED_INDEX: usize = 8;
    pub const WITHDRAWAL_ALLOWED_INDEX: usize = 9;
    pub const MIN_CRANK_INTERVAL_INDEX: usize = 10;
    pub const FEE_INDEX: usize = 11;
    pub const EXTERNAL_FEE_INDEX: usize = 12;

    pub fn new(account: &'a AccountInfo<'b>) -> Self {
        Self {
            key: account.key,
            data: account.data.borrow_mut(),
        }
    }

    pub fn init(&mut self, refdb_name: &ArrayString64) -> ProgramResult {
        if RefDB::is_initialized(&self.data) {
            return Ok(());
        }
        RefDB::init(&mut self.data, refdb_name, ReferenceType::U64)?;

        self.init_refdb_field(
            VaultInfo::CRANK_TIME_INDEX,
            "CrankTime",
            Reference::U64 {
                data: clock::get_time_as_u64()?,
            },
        )?;
        self.init_refdb_field(
            VaultInfo::CRANK_STEP_INDEX,
            "CrankStep",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::TOKEN_A_ADDED_INDEX,
            "TokenAAdded",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::TOKEN_A_REMOVED_INDEX,
            "TokenARemoved",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::TOKEN_B_ADDED_INDEX,
            "TokenBAdded",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::TOKEN_B_REMOVED_INDEX,
            "TokenBRemoved",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::TOKEN_A_REWARDS_INDEX,
            "TokenARewards",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::TOKEN_B_REWARDS_INDEX,
            "TokenBRewards",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::DEPOSIT_ALLOWED_INDEX,
            "DepositAllowed",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::WITHDRAWAL_ALLOWED_INDEX,
            "WithdrawalAllowed",
            Reference::U64 { data: 0 },
        )?;
        self.init_refdb_field(
            VaultInfo::MIN_CRANK_INTERVAL_INDEX,
            "MinCrankInterval",
            Reference::U64 {
                data: VaultInfo::default_min_crank_interval(),
            },
        )?;
        self.init_refdb_field(
            VaultInfo::FEE_INDEX,
            "Fee",
            Reference::U64 {
                data: VaultInfo::default_fee().to_bits(),
            },
        )?;
        self.init_refdb_field(
            VaultInfo::EXTERNAL_FEE_INDEX,
            "ExternalFee",
            Reference::U64 {
                data: VaultInfo::default_external_fee().to_bits(),
            },
        )
    }

    pub fn update_crank_time(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::CRANK_TIME_INDEX,
            &Reference::U64 {
                data: clock::get_time_as_u64()?,
            },
        )
        .map(|_| ())
    }

    pub fn set_crank_step(&mut self, step: u64) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::CRANK_STEP_INDEX,
            &Reference::U64 { data: step },
        )
        .map(|_| ())
    }

    pub fn set_min_crank_interval(&mut self, min_crank_interval_sec: u64) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::MIN_CRANK_INTERVAL_INDEX,
            &Reference::U64 {
                data: min_crank_interval_sec,
            },
        )
        .map(|_| ())
    }

    pub fn set_fee(&mut self, fee: f64) -> ProgramResult {
        if !(0.0..=1.0).contains(&fee) {
            return Err(ProgramError::InvalidArgument);
        }
        RefDB::update_at(
            &mut self.data,
            VaultInfo::FEE_INDEX,
            &Reference::U64 {
                data: fee.to_bits(),
            },
        )
        .map(|_| ())
    }

    pub fn set_external_fee(&mut self, external_fee: f64) -> ProgramResult {
        if !(0.0..=1.0).contains(&external_fee) {
            return Err(ProgramError::InvalidArgument);
        }
        RefDB::update_at(
            &mut self.data,
            VaultInfo::EXTERNAL_FEE_INDEX,
            &Reference::U64 {
                data: external_fee.to_bits(),
            },
        )
        .map(|_| ())
    }

    pub fn enable_deposit(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::DEPOSIT_ALLOWED_INDEX,
            &Reference::U64 { data: 1 },
        )
        .map(|_| ())
    }

    pub fn disable_deposit(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::DEPOSIT_ALLOWED_INDEX,
            &Reference::U64 { data: 0 },
        )
        .map(|_| ())
    }

    pub fn enable_withdrawal(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::WITHDRAWAL_ALLOWED_INDEX,
            &Reference::U64 { data: 1 },
        )
        .map(|_| ())
    }

    pub fn disable_withdrawal(&mut self) -> ProgramResult {
        RefDB::update_at(
            &mut self.data,
            VaultInfo::WITHDRAWAL_ALLOWED_INDEX,
            &Reference::U64 { data: 0 },
        )
        .map(|_| ())
    }

    pub fn add_liquidity(&mut self, token_a_added: u64, token_b_added: u64) -> ProgramResult {
        if token_a_added > 0 {
            let mut token_a_balance = token_a_added;
            if let Some(token_a_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_A_ADDED_INDEX)? {
                if let Reference::U64 { data } = token_a_rec.reference {
                    token_a_balance = token_a_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                VaultInfo::TOKEN_A_ADDED_INDEX,
                &Reference::U64 {
                    data: token_a_balance,
                },
            )?;
        }
        if token_b_added > 0 {
            let mut token_b_balance = token_b_added;
            if let Some(token_b_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_B_ADDED_INDEX)? {
                if let Reference::U64 { data } = token_b_rec.reference {
                    token_b_balance = token_b_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                VaultInfo::TOKEN_B_ADDED_INDEX,
                &Reference::U64 {
                    data: token_b_balance,
                },
            )?;
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
            if let Some(token_a_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_A_REMOVED_INDEX)?
            {
                if let Reference::U64 { data } = token_a_rec.reference {
                    token_a_balance = token_a_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                VaultInfo::TOKEN_A_REMOVED_INDEX,
                &Reference::U64 {
                    data: token_a_balance,
                },
            )?;
        }
        if token_b_removed > 0 {
            let mut token_b_balance = token_b_removed;
            if let Some(token_b_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_B_REMOVED_INDEX)?
            {
                if let Reference::U64 { data } = token_b_rec.reference {
                    token_b_balance = token_b_balance.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                VaultInfo::TOKEN_B_REMOVED_INDEX,
                &Reference::U64 {
                    data: token_b_balance,
                },
            )?;
        }
        Ok(())
    }

    pub fn add_rewards(&mut self, token_a_rewards: u64, token_b_rewards: u64) -> ProgramResult {
        if token_a_rewards > 0 {
            let mut token_a_total = token_a_rewards;
            if let Some(token_a_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_A_REWARDS_INDEX)?
            {
                if let Reference::U64 { data } = token_a_rec.reference {
                    token_a_total = token_a_total.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                VaultInfo::TOKEN_A_REWARDS_INDEX,
                &Reference::U64 {
                    data: token_a_total,
                },
            )?;
        }
        if token_b_rewards > 0 {
            let mut token_b_total = token_b_rewards;
            if let Some(token_b_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_B_REWARDS_INDEX)?
            {
                if let Reference::U64 { data } = token_b_rec.reference {
                    token_b_total = token_b_total.wrapping_add(data);
                }
            }
            RefDB::update_at(
                &mut self.data,
                VaultInfo::TOKEN_B_REWARDS_INDEX,
                &Reference::U64 {
                    data: token_b_total,
                },
            )?;
        }
        Ok(())
    }

    pub fn get_crank_time(&self) -> Result<UnixTimestamp, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::CRANK_TIME_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data as UnixTimestamp);
            }
        }
        Ok(0)
    }

    pub fn get_crank_step(&self) -> Result<u64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::CRANK_STEP_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
    }

    pub fn get_min_crank_interval(&self) -> Result<i64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::MIN_CRANK_INTERVAL_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data as i64);
            }
        }
        Ok(0)
    }

    pub fn get_fee(&self) -> Result<f64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::FEE_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(f64::from_bits(data));
            }
        }
        Ok(0.0)
    }

    pub fn get_external_fee(&self) -> Result<f64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::EXTERNAL_FEE_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(f64::from_bits(data));
            }
        }
        Ok(0.0)
    }

    pub fn is_deposit_allowed(&self) -> Result<bool, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::DEPOSIT_ALLOWED_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data > 0);
            }
        }
        Ok(false)
    }

    pub fn is_withdrawal_allowed(&self) -> Result<bool, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::WITHDRAWAL_ALLOWED_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data > 0);
            }
        }
        Ok(false)
    }

    pub fn get_token_a_added(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_A_ADDED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
    }

    pub fn get_token_b_added(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_B_ADDED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
    }

    pub fn get_token_a_removed(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_A_REMOVED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
    }

    pub fn get_token_b_removed(&self) -> Result<u64, ProgramError> {
        if let Some(deposit_rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_B_REMOVED_INDEX)? {
            if let Reference::U64 { data } = deposit_rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
    }

    pub fn get_token_a_rewards(&self) -> Result<u64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_A_REWARDS_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
    }

    pub fn get_token_b_rewards(&self) -> Result<u64, ProgramError> {
        if let Some(rec) = RefDB::read_at(&self.data, VaultInfo::TOKEN_B_REWARDS_INDEX)? {
            if let Reference::U64 { data } = rec.reference {
                return Ok(data);
            }
        }
        Ok(0)
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
