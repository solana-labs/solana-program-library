use solana_program::instruction::Instruction;
use solend_program::instruction::{
    refresh_obligation, refresh_reserve, withdraw_obligation_collateral,
};
use solend_program::state::{Obligation, Reserve};

use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashSet;

pub struct SolendState {
    lending_program_id: Pubkey,
    obligation_pubkey: Pubkey,
    obligation: Obligation,
    reserves: Vec<(Pubkey, Reserve)>,
}

impl SolendState {
    pub fn new(
        lending_program_id: Pubkey,
        obligation_pubkey: Pubkey,
        rpc_client: &RpcClient,
    ) -> Self {
        let obligation = {
            let data = rpc_client.get_account(&obligation_pubkey).unwrap();
            Obligation::unpack(&data.data).unwrap()
        };

        // get reserve pubkeys
        let reserve_pubkeys: Vec<Pubkey> = {
            let mut r = HashSet::new();
            r.extend(obligation.deposits.iter().map(|d| d.deposit_reserve));
            r.extend(obligation.borrows.iter().map(|b| b.borrow_reserve));
            r.into_iter().collect()
        };

        // get reserve accounts
        let reserves: Vec<(Pubkey, Reserve)> = rpc_client
            .get_multiple_accounts(&reserve_pubkeys)
            .unwrap()
            .into_iter()
            .zip(reserve_pubkeys.iter())
            .map(|(account, pubkey)| (*pubkey, Reserve::unpack(&account.unwrap().data).unwrap()))
            .collect();

        assert!(reserve_pubkeys.len() == reserves.len());

        Self {
            lending_program_id,
            obligation_pubkey,
            obligation,
            reserves,
        }
    }

    pub fn find_reserve_by_key(&self, pubkey: Pubkey) -> Option<&Reserve> {
        self.reserves.iter().find_map(
            |(p, reserve)| {
                if pubkey == *p {
                    Some(reserve)
                } else {
                    None
                }
            },
        )
    }

    fn get_refresh_instructions(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        instructions.extend(self.reserves.iter().map(|(pubkey, reserve)| {
            refresh_reserve(
                self.lending_program_id,
                *pubkey,
                reserve.liquidity.pyth_oracle_pubkey,
                reserve.liquidity.switchboard_oracle_pubkey,
            )
        }));

        let reserve_pubkeys: Vec<Pubkey> = {
            let mut r = Vec::new();
            r.extend(self.obligation.deposits.iter().map(|d| d.deposit_reserve));
            r.extend(self.obligation.borrows.iter().map(|b| b.borrow_reserve));
            r
        };

        // refresh obligation
        instructions.push(refresh_obligation(
            self.lending_program_id,
            self.obligation_pubkey,
            reserve_pubkeys,
        ));

        instructions
    }

    /// withdraw obligation ctokens to owner's ata
    pub fn withdraw(
        &self,
        withdraw_reserve_pubkey: &Pubkey,
        collateral_amount: u64,
    ) -> Vec<Instruction> {
        let mut instructions = self.get_refresh_instructions();

        // find repay, withdraw reserve states
        let withdraw_reserve = self
            .reserves
            .iter()
            .find_map(|(pubkey, reserve)| {
                if withdraw_reserve_pubkey == pubkey {
                    Some(reserve)
                } else {
                    None
                }
            })
            .unwrap();

        instructions.push(withdraw_obligation_collateral(
            self.lending_program_id,
            collateral_amount,
            withdraw_reserve.collateral.supply_pubkey,
            get_associated_token_address(
                &self.obligation.owner,
                &withdraw_reserve.collateral.mint_pubkey,
            ),
            *withdraw_reserve_pubkey,
            self.obligation_pubkey,
            withdraw_reserve.lending_market,
            self.obligation.owner,
        ));

        instructions
    }
}
