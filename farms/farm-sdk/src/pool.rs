//! Liquidity Pools

use {
    crate::{pack::*, string::ArrayString64, traits::*},
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_program::program_error::ProgramError,
    solana_program::pubkey::Pubkey,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolRoute {
    Raydium {
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        amm_id: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        amm_authority: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        amm_open_orders: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        amm_target: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        pool_withdraw_queue: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        pool_temp_lp_token_account: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        serum_program_id: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        serum_market: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        serum_coin_vault_account: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        serum_pc_vault_account: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        serum_vault_signer: Pubkey,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        serum_bids: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        serum_asks: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        serum_event_queue: Option<Pubkey>,
    },
    Saber {
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        swap_account: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        swap_authority: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        fees_account_a: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        fees_account_b: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        decimal_wrapper_program: Pubkey,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        wrapped_token_a_ref: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        wrapped_token_a_vault: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        decimal_wrapper_token_a: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        wrapped_token_b_ref: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        wrapped_token_b_vault: Option<Pubkey>,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        decimal_wrapper_token_b: Option<Pubkey>,
    },
    Orca {
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        amm_id: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        amm_authority: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        fees_account: Pubkey,
    },
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum PoolRouteType {
    Raydium,
    Saber,
    Orca,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum PoolType {
    Amm,
    AmmStable,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum PoolTokenType {
    VaultToken,
    PoolToken,
    FarmToken,
    Token,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pool {
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
    pub version: u16,
    pub pool_type: PoolType,
    pub official: bool,
    pub refdb_index: Option<u32>,
    pub refdb_counter: u16,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub token_a_ref: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub token_b_ref: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub lp_token_ref: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub token_a_account: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub token_b_account: Option<Pubkey>,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub router_program_id: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub pool_program_id: Pubkey,
    pub route: PoolRoute,
}

impl Named for Pool {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Versioned for Pool {
    fn version(&self) -> u16 {
        self.version
    }
}

impl Pool {
    pub const MAX_LEN: usize = 756;
    pub const RAYDIUM_POOL_LEN: usize = 756;
    pub const SABER_POOL_LEN: usize = 663;
    pub const ORCA_POOL_LEN: usize = 401;

    pub fn get_size(&self) -> usize {
        match self.route {
            PoolRoute::Raydium { .. } => Pool::RAYDIUM_POOL_LEN,
            PoolRoute::Saber { .. } => Pool::SABER_POOL_LEN,
            PoolRoute::Orca { .. } => Pool::ORCA_POOL_LEN,
        }
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self.route {
            PoolRoute::Raydium { .. } => self.pack_raydium(output),
            PoolRoute::Saber { .. } => self.pack_saber(output),
            PoolRoute::Orca { .. } => self.pack_orca(output),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Pool::MAX_LEN] = [0; Pool::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<Pool, ProgramError> {
        check_data_len(input, 1)?;
        let pool_route_type = PoolRouteType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidAccountData))?;
        match pool_route_type {
            PoolRouteType::Raydium => Pool::unpack_raydium(input),
            PoolRouteType::Saber => Pool::unpack_saber(input),
            PoolRouteType::Orca => Pool::unpack_orca(input),
        }
    }

    fn pack_raydium(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Pool::RAYDIUM_POOL_LEN)?;

        if let PoolRoute::Raydium {
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target,
            pool_withdraw_queue,
            pool_temp_lp_token_account,
            serum_program_id,
            serum_market,
            serum_coin_vault_account,
            serum_pc_vault_account,
            serum_vault_signer,
            serum_bids,
            serum_asks,
            serum_event_queue,
        } = self.route
        {
            let output = array_mut_ref![output, 0, Pool::RAYDIUM_POOL_LEN];

            let (
                pool_route_type_out,
                name_out,
                version_out,
                pool_type_out,
                official_out,
                refdb_index_out,
                refdb_counter_out,
                token_a_ref_out,
                token_b_ref_out,
                lp_token_ref_out,
                token_a_account_out,
                token_b_account_out,
                router_program_id_out,
                pool_program_id_out,
                amm_id_out,
                amm_authority_out,
                amm_open_orders_out,
                amm_target_out,
                pool_withdraw_queue_out,
                pool_temp_lp_token_account_out,
                serum_program_id_out,
                serum_market_out,
                serum_coin_vault_account_out,
                serum_pc_vault_account_out,
                serum_vault_signer_out,
                serum_bids_out,
                serum_asks_out,
                serum_event_queue_out,
            ) = mut_array_refs![
                output, 1, 64, 2, 1, 1, 5, 2, 33, 33, 33, 33, 33, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 33, 33, 33
            ];

            pool_route_type_out[0] = PoolRouteType::Raydium as u8;

            pack_array_string64(&self.name, name_out);
            *version_out = self.version.to_le_bytes();
            pool_type_out[0] = self.pool_type as u8;
            official_out[0] = self.official as u8;
            pack_option_u32(self.refdb_index, refdb_index_out);
            *refdb_counter_out = self.refdb_counter.to_le_bytes();
            pack_option_key(&self.token_a_ref, token_a_ref_out);
            pack_option_key(&self.token_b_ref, token_b_ref_out);
            pack_option_key(&self.lp_token_ref, lp_token_ref_out);
            pack_option_key(&self.token_a_account, token_a_account_out);
            pack_option_key(&self.token_b_account, token_b_account_out);
            router_program_id_out.copy_from_slice(self.router_program_id.as_ref());
            pool_program_id_out.copy_from_slice(self.pool_program_id.as_ref());
            amm_id_out.copy_from_slice(amm_id.as_ref());
            amm_authority_out.copy_from_slice(amm_authority.as_ref());
            amm_open_orders_out.copy_from_slice(amm_open_orders.as_ref());
            amm_target_out.copy_from_slice(amm_target.as_ref());
            pool_withdraw_queue_out.copy_from_slice(pool_withdraw_queue.as_ref());
            pool_temp_lp_token_account_out.copy_from_slice(pool_temp_lp_token_account.as_ref());
            serum_program_id_out.copy_from_slice(serum_program_id.as_ref());
            serum_market_out.copy_from_slice(serum_market.as_ref());
            serum_coin_vault_account_out.copy_from_slice(serum_coin_vault_account.as_ref());
            serum_pc_vault_account_out.copy_from_slice(serum_pc_vault_account.as_ref());
            serum_vault_signer_out.copy_from_slice(serum_vault_signer.as_ref());
            pack_option_key(&serum_bids, serum_bids_out);
            pack_option_key(&serum_asks, serum_asks_out);
            pack_option_key(&serum_event_queue, serum_event_queue_out);

            Ok(Pool::RAYDIUM_POOL_LEN)
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn pack_saber(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Pool::SABER_POOL_LEN)?;

        if let PoolRoute::Saber {
            swap_account,
            swap_authority,
            fees_account_a,
            fees_account_b,
            decimal_wrapper_program,
            wrapped_token_a_ref,
            wrapped_token_a_vault,
            decimal_wrapper_token_a,
            wrapped_token_b_ref,
            wrapped_token_b_vault,
            decimal_wrapper_token_b,
        } = self.route
        {
            let output = array_mut_ref![output, 0, Pool::SABER_POOL_LEN];

            let (
                pool_route_type_out,
                name_out,
                version_out,
                pool_type_out,
                official_out,
                refdb_index_out,
                refdb_counter_out,
                token_a_ref_out,
                token_b_ref_out,
                lp_token_ref_out,
                token_a_account_out,
                token_b_account_out,
                router_program_id_out,
                pool_program_id_out,
                swap_account_out,
                swap_authority_out,
                fees_account_a_out,
                fees_account_b_out,
                decimal_wrapper_program_out,
                wrapped_token_a_ref_out,
                wrapped_token_a_vault_out,
                decimal_wrapper_token_a_out,
                wrapped_token_b_ref_out,
                wrapped_token_b_vault_out,
                decimal_wrapper_token_b_out,
            ) = mut_array_refs![
                output, 1, 64, 2, 1, 1, 5, 2, 33, 33, 33, 33, 33, 32, 32, 32, 32, 32, 32, 32, 33,
                33, 33, 33, 33, 33
            ];

            pool_route_type_out[0] = PoolRouteType::Saber as u8;

            pack_array_string64(&self.name, name_out);
            *version_out = self.version.to_le_bytes();
            pool_type_out[0] = self.pool_type as u8;
            official_out[0] = self.official as u8;
            pack_option_u32(self.refdb_index, refdb_index_out);
            *refdb_counter_out = self.refdb_counter.to_le_bytes();
            pack_option_key(&self.token_a_ref, token_a_ref_out);
            pack_option_key(&self.token_b_ref, token_b_ref_out);
            pack_option_key(&self.lp_token_ref, lp_token_ref_out);
            pack_option_key(&self.token_a_account, token_a_account_out);
            pack_option_key(&self.token_b_account, token_b_account_out);
            router_program_id_out.copy_from_slice(self.router_program_id.as_ref());
            pool_program_id_out.copy_from_slice(self.pool_program_id.as_ref());
            swap_account_out.copy_from_slice(swap_account.as_ref());
            swap_authority_out.copy_from_slice(swap_authority.as_ref());
            fees_account_a_out.copy_from_slice(fees_account_a.as_ref());
            fees_account_b_out.copy_from_slice(fees_account_b.as_ref());
            decimal_wrapper_program_out.copy_from_slice(decimal_wrapper_program.as_ref());
            pack_option_key(&wrapped_token_a_ref, wrapped_token_a_ref_out);
            pack_option_key(&wrapped_token_a_vault, wrapped_token_a_vault_out);
            pack_option_key(&decimal_wrapper_token_a, decimal_wrapper_token_a_out);
            pack_option_key(&wrapped_token_b_ref, wrapped_token_b_ref_out);
            pack_option_key(&wrapped_token_b_vault, wrapped_token_b_vault_out);
            pack_option_key(&decimal_wrapper_token_b, decimal_wrapper_token_b_out);

            Ok(Pool::SABER_POOL_LEN)
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn pack_orca(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Pool::ORCA_POOL_LEN)?;

        if let PoolRoute::Orca {
            amm_id,
            amm_authority,
            fees_account,
        } = self.route
        {
            let output = array_mut_ref![output, 0, Pool::ORCA_POOL_LEN];

            let (
                pool_route_type_out,
                name_out,
                version_out,
                pool_type_out,
                official_out,
                refdb_index_out,
                refdb_counter_out,
                token_a_ref_out,
                token_b_ref_out,
                lp_token_ref_out,
                token_a_account_out,
                token_b_account_out,
                router_program_id_out,
                pool_program_id_out,
                amm_id_out,
                amm_authority_out,
                fees_account_out,
            ) = mut_array_refs![
                output, 1, 64, 2, 1, 1, 5, 2, 33, 33, 33, 33, 33, 32, 32, 32, 32, 32
            ];

            pool_route_type_out[0] = PoolRouteType::Orca as u8;

            pack_array_string64(&self.name, name_out);
            *version_out = self.version.to_le_bytes();
            pool_type_out[0] = self.pool_type as u8;
            official_out[0] = self.official as u8;
            pack_option_u32(self.refdb_index, refdb_index_out);
            *refdb_counter_out = self.refdb_counter.to_le_bytes();
            pack_option_key(&self.token_a_ref, token_a_ref_out);
            pack_option_key(&self.token_b_ref, token_b_ref_out);
            pack_option_key(&self.lp_token_ref, lp_token_ref_out);
            pack_option_key(&self.token_a_account, token_a_account_out);
            pack_option_key(&self.token_b_account, token_b_account_out);
            router_program_id_out.copy_from_slice(self.router_program_id.as_ref());
            pool_program_id_out.copy_from_slice(self.pool_program_id.as_ref());
            amm_id_out.copy_from_slice(amm_id.as_ref());
            amm_authority_out.copy_from_slice(amm_authority.as_ref());
            fees_account_out.copy_from_slice(fees_account.as_ref());

            Ok(Pool::ORCA_POOL_LEN)
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack_raydium(input: &[u8]) -> Result<Pool, ProgramError> {
        check_data_len(input, Pool::RAYDIUM_POOL_LEN)?;

        let input = array_ref![input, 1, Pool::RAYDIUM_POOL_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            version,
            pool_type,
            official,
            refdb_index,
            refdb_counter,
            token_a_ref,
            token_b_ref,
            lp_token_ref,
            token_a_account,
            token_b_account,
            router_program_id,
            pool_program_id,
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target,
            pool_withdraw_queue,
            pool_temp_lp_token_account,
            serum_program_id,
            serum_market,
            serum_coin_vault_account,
            serum_pc_vault_account,
            serum_vault_signer,
            serum_bids,
            serum_asks,
            serum_event_queue,
        ) = array_refs![
            input, 64, 2, 1, 1, 5, 2, 33, 33, 33, 33, 33, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 33, 33, 33
        ];

        Ok(Self {
            name: unpack_array_string64(name)?,
            version: u16::from_le_bytes(*version),
            pool_type: PoolType::try_from_primitive(pool_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            official: unpack_bool(official)?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            token_a_ref: unpack_option_key(token_a_ref)?,
            token_b_ref: unpack_option_key(token_b_ref)?,
            lp_token_ref: unpack_option_key(lp_token_ref)?,
            token_a_account: unpack_option_key(token_a_account)?,
            token_b_account: unpack_option_key(token_b_account)?,
            router_program_id: Pubkey::new_from_array(*router_program_id),
            pool_program_id: Pubkey::new_from_array(*pool_program_id),
            route: PoolRoute::Raydium {
                amm_id: Pubkey::new_from_array(*amm_id),
                amm_authority: Pubkey::new_from_array(*amm_authority),
                amm_open_orders: Pubkey::new_from_array(*amm_open_orders),
                amm_target: Pubkey::new_from_array(*amm_target),
                pool_withdraw_queue: Pubkey::new_from_array(*pool_withdraw_queue),
                pool_temp_lp_token_account: Pubkey::new_from_array(*pool_temp_lp_token_account),
                serum_program_id: Pubkey::new_from_array(*serum_program_id),
                serum_market: Pubkey::new_from_array(*serum_market),
                serum_coin_vault_account: Pubkey::new_from_array(*serum_coin_vault_account),
                serum_pc_vault_account: Pubkey::new_from_array(*serum_pc_vault_account),
                serum_vault_signer: Pubkey::new_from_array(*serum_vault_signer),
                serum_bids: unpack_option_key(serum_bids)?,
                serum_asks: unpack_option_key(serum_asks)?,
                serum_event_queue: unpack_option_key(serum_event_queue)?,
            },
        })
    }

    fn unpack_saber(input: &[u8]) -> Result<Pool, ProgramError> {
        check_data_len(input, Pool::SABER_POOL_LEN)?;

        let input = array_ref![input, 1, Pool::SABER_POOL_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            version,
            pool_type,
            official,
            refdb_index,
            refdb_counter,
            token_a_ref,
            token_b_ref,
            lp_token_ref,
            token_a_account,
            token_b_account,
            router_program_id,
            pool_program_id,
            swap_account,
            swap_authority,
            fees_account_a,
            fees_account_b,
            decimal_wrapper_program,
            wrapped_token_a_ref,
            wrapped_token_a_vault,
            decimal_wrapper_token_a,
            wrapped_token_b_ref,
            wrapped_token_b_vault,
            decimal_wrapper_token_b,
        ) = array_refs![
            input, 64, 2, 1, 1, 5, 2, 33, 33, 33, 33, 33, 32, 32, 32, 32, 32, 32, 32, 33, 33, 33,
            33, 33, 33
        ];

        Ok(Self {
            name: unpack_array_string64(name)?,
            version: u16::from_le_bytes(*version),
            pool_type: PoolType::try_from_primitive(pool_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            official: unpack_bool(official)?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            token_a_ref: unpack_option_key(token_a_ref)?,
            token_b_ref: unpack_option_key(token_b_ref)?,
            lp_token_ref: unpack_option_key(lp_token_ref)?,
            token_a_account: unpack_option_key(token_a_account)?,
            token_b_account: unpack_option_key(token_b_account)?,
            router_program_id: Pubkey::new_from_array(*router_program_id),
            pool_program_id: Pubkey::new_from_array(*pool_program_id),
            route: PoolRoute::Saber {
                swap_account: Pubkey::new_from_array(*swap_account),
                swap_authority: Pubkey::new_from_array(*swap_authority),
                fees_account_a: Pubkey::new_from_array(*fees_account_a),
                fees_account_b: Pubkey::new_from_array(*fees_account_b),
                decimal_wrapper_program: Pubkey::new_from_array(*decimal_wrapper_program),
                wrapped_token_a_ref: unpack_option_key(wrapped_token_a_ref)?,
                wrapped_token_a_vault: unpack_option_key(wrapped_token_a_vault)?,
                decimal_wrapper_token_a: unpack_option_key(decimal_wrapper_token_a)?,
                wrapped_token_b_ref: unpack_option_key(wrapped_token_b_ref)?,
                wrapped_token_b_vault: unpack_option_key(wrapped_token_b_vault)?,
                decimal_wrapper_token_b: unpack_option_key(decimal_wrapper_token_b)?,
            },
        })
    }

    fn unpack_orca(input: &[u8]) -> Result<Pool, ProgramError> {
        check_data_len(input, Pool::ORCA_POOL_LEN)?;

        let input = array_ref![input, 1, Pool::ORCA_POOL_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            version,
            pool_type,
            official,
            refdb_index,
            refdb_counter,
            token_a_ref,
            token_b_ref,
            lp_token_ref,
            token_a_account,
            token_b_account,
            router_program_id,
            pool_program_id,
            amm_id,
            amm_authority,
            fees_account,
        ) = array_refs![input, 64, 2, 1, 1, 5, 2, 33, 33, 33, 33, 33, 32, 32, 32, 32, 32];

        Ok(Self {
            name: unpack_array_string64(name)?,
            version: u16::from_le_bytes(*version),
            pool_type: PoolType::try_from_primitive(pool_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            official: unpack_bool(official)?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            token_a_ref: unpack_option_key(token_a_ref)?,
            token_b_ref: unpack_option_key(token_b_ref)?,
            lp_token_ref: unpack_option_key(lp_token_ref)?,
            token_a_account: unpack_option_key(token_a_account)?,
            token_b_account: unpack_option_key(token_b_account)?,
            router_program_id: Pubkey::new_from_array(*router_program_id),
            pool_program_id: Pubkey::new_from_array(*pool_program_id),
            route: PoolRoute::Orca {
                amm_id: Pubkey::new_from_array(*amm_id),
                amm_authority: Pubkey::new_from_array(*amm_authority),
                fees_account: Pubkey::new_from_array(*fees_account),
            },
        })
    }
}

impl std::fmt::Display for PoolType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            PoolType::Amm => write!(f, "Amm"),
            PoolType::AmmStable => write!(f, "AmmStable"),
        }
    }
}

impl std::fmt::Display for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_serialization() {
        let ri1 = Pool {
            name: ArrayString64::from_utf8("test").unwrap(),
            version: 2,
            pool_type: PoolType::Amm,
            official: true,
            refdb_index: Some(1),
            refdb_counter: 2,
            token_a_ref: Some(Pubkey::new_unique()),
            token_b_ref: Some(Pubkey::new_unique()),
            lp_token_ref: Some(Pubkey::new_unique()),
            token_a_account: None,
            token_b_account: None,
            router_program_id: Pubkey::new_unique(),
            pool_program_id: Pubkey::new_unique(),
            route: PoolRoute::Raydium {
                amm_id: Pubkey::new_unique(),
                amm_authority: Pubkey::new_unique(),
                amm_open_orders: Pubkey::new_unique(),
                amm_target: Pubkey::new_unique(),
                pool_withdraw_queue: Pubkey::new_unique(),
                pool_temp_lp_token_account: Pubkey::new_unique(),
                serum_program_id: Pubkey::new_unique(),
                serum_market: Pubkey::new_unique(),
                serum_coin_vault_account: Pubkey::new_unique(),
                serum_pc_vault_account: Pubkey::new_unique(),
                serum_vault_signer: Pubkey::new_unique(),
                serum_bids: Some(Pubkey::new_unique()),
                serum_asks: Some(Pubkey::new_unique()),
                serum_event_queue: Some(Pubkey::new_unique()),
            },
        };

        let vec = ri1.to_vec().unwrap();

        let ri2 = Pool::unpack(&vec[..]).unwrap();

        assert_eq!(ri1, ri2);
    }
}
