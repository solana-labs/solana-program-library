//! Solana Farm Client Governance Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        id::{DAO_PROGRAM_NAME, DAO_TOKEN_NAME},
        token::Token,
    },
    solana_sdk::{borsh::try_from_slice_unchecked, instruction::Instruction, pubkey::Pubkey},
    spl_governance::instruction as dao_instruction,
    spl_governance::state::{
        governance::GovernanceConfig,
        proposal::{get_proposal_address, VoteType},
        proposal_instruction::{
            get_proposal_instruction_address, InstructionData, ProposalInstructionV2,
        },
        realm::get_realm_address,
        token_owner_record::get_token_owner_record_address,
        vote_record::{Vote, VoteChoice},
    },
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new instruction for tokens deposit to the farms realm
    pub fn new_instruction_governance_tokens_deposit(
        &self,
        wallet_address: &Pubkey,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let dao_program = self.get_program_id(DAO_PROGRAM_NAME)?;
        let realm_address = get_realm_address(&dao_program, DAO_PROGRAM_NAME);
        let dao_token = self.get_token(DAO_TOKEN_NAME)?;
        let token_addr = self.get_associated_token_address(wallet_address, DAO_TOKEN_NAME)?;

        let inst = dao_instruction::deposit_governing_tokens(
            &dao_program,
            &realm_address,
            &token_addr,
            wallet_address,
            wallet_address,
            wallet_address,
            self.ui_amount_to_tokens_with_decimals(ui_amount, dao_token.decimals),
            &dao_token.mint,
        );

        Ok(inst)
    }

    /// Creates a new instruction for tokens withdrawal from the farms realm
    pub fn new_instruction_governance_tokens_withdraw(
        &self,
        wallet_address: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        let dao_program = self.get_program_id(DAO_PROGRAM_NAME)?;
        let realm_address = get_realm_address(&dao_program, DAO_PROGRAM_NAME);
        let dao_token = self.get_token(DAO_TOKEN_NAME)?;
        let token_addr = self.get_associated_token_address(wallet_address, DAO_TOKEN_NAME)?;

        let inst = dao_instruction::withdraw_governing_tokens(
            &dao_program,
            &realm_address,
            &token_addr,
            wallet_address,
            &dao_token.mint,
        );

        Ok(inst)
    }

    /// Creates a new instruction for initializing a new governance proposal
    pub fn new_instruction_governance_proposal_new(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_name: &str,
        proposal_link: &str,
        proposal_index: u32,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, realm_address, dao_token, governance, token_owner, _proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst = dao_instruction::create_proposal(
            &dao_program,
            &governance,
            &token_owner,
            wallet_address,
            wallet_address,
            None,
            &realm_address,
            proposal_name.to_string(),
            proposal_link.to_string(),
            &dao_token.mint,
            VoteType::SingleChoice,
            vec![proposal_name.to_string()],
            true,
            proposal_index,
        );

        Ok(inst)
    }

    /// Creates a new instruction for canceling governance proposal
    pub fn new_instruction_governance_proposal_cancel(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst = dao_instruction::cancel_proposal(
            &dao_program,
            &proposal_address,
            &token_owner,
            wallet_address,
            &governance,
        );

        Ok(inst)
    }

    /// Creates a new instruction for adding a signatory to governance proposal
    pub fn new_instruction_governance_signatory_add(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        signatory: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, _governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst = dao_instruction::add_signatory(
            &dao_program,
            &proposal_address,
            &token_owner,
            wallet_address,
            wallet_address,
            signatory,
        );

        Ok(inst)
    }

    /// Creates a new instruction for removing the signatory from governance proposal
    pub fn new_instruction_governance_signatory_remove(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        signatory: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, _governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst = dao_instruction::remove_signatory(
            &dao_program,
            &proposal_address,
            &token_owner,
            wallet_address,
            signatory,
            wallet_address,
        );

        Ok(inst)
    }

    /// Creates a new instruction for signing off governance proposal
    pub fn new_instruction_governance_sign_off(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, _governance, _token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst =
            dao_instruction::sign_off_proposal(&dao_program, &proposal_address, wallet_address);

        Ok(inst)
    }

    /// Creates a new instruction for casting a vote on governance proposal
    pub fn new_instruction_governance_vote_cast(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        vote: u8,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, realm_address, dao_token, governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let voter_token_owner = get_token_owner_record_address(
            &dao_program,
            &realm_address,
            &dao_token.mint,
            wallet_address,
        );

        let inst = dao_instruction::cast_vote(
            &dao_program,
            &realm_address,
            &governance,
            &proposal_address,
            &token_owner,
            &voter_token_owner,
            wallet_address,
            &dao_token.mint,
            wallet_address,
            None,
            if vote > 0 {
                Vote::Approve(vec![VoteChoice {
                    rank: 0,
                    weight_percentage: 100,
                }])
            } else {
                Vote::Deny
            },
        );

        Ok(inst)
    }

    /// Creates a new instruction for removing the vote from governance proposal
    pub fn new_instruction_governance_vote_relinquish(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, dao_token, governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst = dao_instruction::relinquish_vote(
            &dao_program,
            &governance,
            &proposal_address,
            &token_owner,
            &dao_token.mint,
            Some(*wallet_address),
            Some(*wallet_address),
        );

        Ok(inst)
    }

    /// Creates a new instruction for finalizing the vote on governance proposal
    pub fn new_instruction_governance_vote_finalize(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, realm_address, dao_token, governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let inst = dao_instruction::finalize_vote(
            &dao_program,
            &realm_address,
            &governance,
            &proposal_address,
            &token_owner,
            &dao_token.mint,
        );

        Ok(inst)
    }

    /// Creates a new instruction for adding a new instruction to governance proposal
    pub fn new_instruction_governance_instruction_insert(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
        instruction: &Instruction,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let instruction_data: InstructionData = instruction.clone().into();

        let inst = dao_instruction::insert_instruction(
            &dao_program,
            &governance,
            &proposal_address,
            &token_owner,
            wallet_address,
            wallet_address,
            0,
            instruction_index,
            0,
            instruction_data,
        );

        Ok(inst)
    }

    /// Creates a new instruction for removing the instruction from governance proposal
    pub fn new_instruction_governance_instruction_remove(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, _governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let instruction_address = get_proposal_instruction_address(
            &dao_program,
            &proposal_address,
            &0u16.to_le_bytes(),
            &instruction_index.to_le_bytes(),
        );

        let inst = dao_instruction::remove_instruction(
            &dao_program,
            &proposal_address,
            &token_owner,
            wallet_address,
            &instruction_address,
            wallet_address,
        );

        Ok(inst)
    }

    /// Creates a new instruction for executing the instruction in governance proposal
    pub fn new_instruction_governance_instruction_execute(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, governance, _token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let instruction_address = get_proposal_instruction_address(
            &dao_program,
            &proposal_address,
            &0u16.to_le_bytes(),
            &instruction_index.to_le_bytes(),
        );

        let data = self.rpc_client.get_account_data(&instruction_address)?;
        let ins_data: InstructionData =
            try_from_slice_unchecked::<ProposalInstructionV2>(data.as_slice())
                .map_err(|e| FarmClientError::IOError(e.to_string()))?
                .instruction;
        let mut instruction: Instruction = (&ins_data).into();

        for account in &mut instruction.accounts {
            if account.pubkey == governance {
                account.is_signer = false;
            }
        }

        let inst = dao_instruction::execute_instruction(
            &dao_program,
            &governance,
            &proposal_address,
            &instruction_address,
            &instruction.program_id,
            instruction.accounts.as_slice(),
        );

        Ok(inst)
    }

    /// Creates a new instruction for marking the instruction in governance proposal as failed
    pub fn new_instruction_governance_instruction_flag_error(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, _governance, token_owner, proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, proposal_index)?;

        let instruction_address = get_proposal_instruction_address(
            &dao_program,
            &proposal_address,
            &0u16.to_le_bytes(),
            &instruction_index.to_le_bytes(),
        );

        let inst = dao_instruction::flag_instruction_error(
            &dao_program,
            &proposal_address,
            &token_owner,
            wallet_address,
            &instruction_address,
        );

        Ok(inst)
    }

    /// Creates a new instruction for changing the governance config
    pub fn new_instruction_governance_set_config(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        config: &GovernanceConfig,
    ) -> Result<Instruction, FarmClientError> {
        let (dao_program, _realm_address, _dao_token, governance, _token_owner, _proposal_address) =
            self.get_dao_accounts(wallet_address, governance_name, 0)?;

        let inst =
            dao_instruction::set_governance_config(&dao_program, &governance, config.clone());

        Ok(inst)
    }

    /////////////// helpers
    fn get_dao_accounts(
        &self,
        wallet_address: &Pubkey,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<(Pubkey, Pubkey, Token, Pubkey, Pubkey, Pubkey), FarmClientError> {
        let dao_program = self.get_program_id(DAO_PROGRAM_NAME)?;
        let realm_address = get_realm_address(&dao_program, DAO_PROGRAM_NAME);
        let dao_token = self.get_token(DAO_TOKEN_NAME)?;
        let governance = self.governance_get_address(governance_name)?;
        let token_owner = get_token_owner_record_address(
            &dao_program,
            &realm_address,
            &dao_token.mint,
            wallet_address,
        );
        let proposal_address = get_proposal_address(
            &dao_program,
            &governance,
            &dao_token.mint,
            &proposal_index.to_le_bytes(),
        );
        Ok((
            dao_program,
            realm_address,
            dao_token,
            governance,
            token_owner,
            proposal_address,
        ))
    }
}
