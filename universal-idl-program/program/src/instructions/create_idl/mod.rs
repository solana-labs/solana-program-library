use crate::account_checks::assert_account_owned_by_loader;
use crate::account_checks::assert_address;
use crate::account_checks::assert_executable;
use crate::account_checks::assert_mut;
use crate::account_checks::assert_owner;
use crate::account_checks::assert_program_authority_matches_program;
use crate::account_checks::assert_program_data_matches_program;
use crate::account_checks::assert_signer;
use crate::account_checks::assert_system_program;
use crate::account_checks::assert_with_msg;
use crate::account_checks::is_frozen_program;
use crate::account_checks::is_upgradeable_program;
use crate::id;
use crate::state::assert_frozen_authority_seeds;
use crate::state::assert_idl_seeds;
use crate::state::FrozenProgramAuthority;
use crate::state::Idl;
use crate::state::SolanaAccount;
use crate::IdlProgramInstruction;

use borsh::BorshSerialize;
use lazy_format::lazy_format;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::program::invoke_signed;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::create_account,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

pub fn create_idl(
    program_id: Pubkey,
    payer: Pubkey,
    program_authority: Pubkey,
    idl_account: Pubkey,
    program: Pubkey,
    program_data_or_frozen_auth: Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(program_authority, true),
            AccountMeta::new(idl_account, false),
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(program_data_or_frozen_auth, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: IdlProgramInstruction::CreateIdl.try_to_vec()?,
    })
}

pub enum AuthoritySource<'a, 'info> {
    UpgradeableAuthority {
        upgradeable_program: &'a AccountInfo<'info>,
        program_data: UpgradeableLoaderState,
    },
    FrozenProgramAuthority {
        frozen_program: &'a AccountInfo<'info>,
        frozen_authority: FrozenProgramAuthority,
    },
}
impl<'a, 'info> AuthoritySource<'a, 'info> {
    pub fn check_authority(&self, authority: &Pubkey) -> ProgramResult {
        match self {
            Self::FrozenProgramAuthority {
                frozen_authority, ..
            } => assert_address(authority, &frozen_authority.authority, "Frozen authority"),
            Self::UpgradeableAuthority { program_data, .. } => {
                assert_program_authority_matches_program(program_data, authority)
            }
        }
    }

    pub fn load(
        accounts_iter: &mut impl Iterator<Item = &'a AccountInfo<'info>>,
    ) -> Result<Self, ProgramError> {
        let program = next_account_info(accounts_iter)?;
        assert_executable(program, "program")?;
        if is_upgradeable_program(program) {
            let program_data = next_account_info(accounts_iter)?;

            // program
            let program_state: UpgradeableLoaderState =
                bincode::deserialize(&program.data.borrow())
                    .map_err(|_| ProgramError::InvalidAccountData)?;
            assert_program_data_matches_program(&program_state, program_data.key)?;

            // program_data
            assert_account_owned_by_loader(program_data, "program_data")?;
            let program_data_state: UpgradeableLoaderState =
                bincode::deserialize(&program_data.data.borrow())
                    .map_err(|_| ProgramError::InvalidAccountData)?;

            Ok(Self::UpgradeableAuthority {
                upgradeable_program: program,
                program_data: program_data_state,
            })
        } else if is_frozen_program(program) {
            let frozen_authority_account = next_account_info(accounts_iter)?;
            assert_frozen_authority_seeds(program.key, frozen_authority_account.key)?;
            assert_owner(frozen_authority_account, &crate::id(), "frozen_authority")?;

            let frozen_authority: FrozenProgramAuthority =
                FrozenProgramAuthority::from_account_info(frozen_authority_account)?;

            Ok(Self::FrozenProgramAuthority {
                frozen_program: program,
                frozen_authority: frozen_authority,
            })
        } else {
            Err(assert_with_msg(
                false,
                ProgramError::IllegalOwner,
                lazy_format!(
                    "Program account must be owned by upgradeable or frozen loader, instead of: {}",
                    program.owner
                ),
            )
            .unwrap_err())
        }
    }
}

pub struct CreateIdlCtx<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub program_authority: &'a AccountInfo<'info>,
    pub idl_account: &'a AccountInfo<'info>,
    pub authority_source: AuthoritySource<'a, 'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> CreateIdlCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // payer
        let payer = next_account_info(accounts_iter)?;
        assert_signer(payer, "payer")?;
        assert_mut(payer, "payer")?;

        // program authority
        let program_authority = next_account_info(accounts_iter)?;
        assert_signer(program_authority, "program_authority")?;

        // idl_account
        let idl_account = next_account_info(accounts_iter)?;
        assert_mut(idl_account, "idl_account")?;

        // authority_source
        let authority_source = AuthoritySource::load(accounts_iter)?;
        authority_source.check_authority(program_authority.key)?;

        // system_program
        let system_program = next_account_info(accounts_iter)?;
        assert_system_program(system_program)?;

        Ok(Self {
            payer,
            program_authority,
            idl_account,
            authority_source,
            system_program,
        })
    }

    pub fn get_program(&self) -> Pubkey {
        match self.authority_source {
            AuthoritySource::UpgradeableAuthority {
                upgradeable_program,
                ..
            } => upgradeable_program.key.clone(),
            AuthoritySource::FrozenProgramAuthority { frozen_program, .. } => {
                frozen_program.key.clone()
            }
        }
    }
}

pub fn handler(ctx: CreateIdlCtx) -> ProgramResult {
    let idl_seeds = assert_idl_seeds(&ctx.get_program(), ctx.idl_account.key)?;
    let data_len = std::mem::size_of::<Idl>();
    invoke_signed(
        &create_account(
            ctx.payer.key,
            ctx.idl_account.key,
            Rent::get()?.minimum_balance(data_len as usize),
            data_len as u64,
            &id(),
        ),
        &[ctx.payer.clone(), ctx.idl_account.clone()],
        &[&idl_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>()],
    )?;

    let mut idl: Idl = Idl::new();
    idl.authority = *ctx.program_authority.key;
    idl.slot = Clock::get()?.slot;
    idl.save(ctx.idl_account)?;

    Ok(())
}
