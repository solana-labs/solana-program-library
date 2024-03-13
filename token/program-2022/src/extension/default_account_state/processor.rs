use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            default_account_state::{
                instruction::{decode_instruction, DefaultAccountStateInstruction},
                DefaultAccountState,
            },
            BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
        },
        pod::{PodCOption, PodMint},
        processor::Processor,
        state::AccountState,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

fn check_valid_default_state(state: AccountState) -> ProgramResult {
    match state {
        AccountState::Uninitialized => Err(TokenError::InvalidState.into()),
        _ => Ok(()),
    }
}

fn process_initialize_default_account_state(
    accounts: &[AccountInfo],
    state: AccountState,
) -> ProgramResult {
    check_valid_default_state(state)?;
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;
    let extension = mint.init_extension::<DefaultAccountState>(true)?;
    extension.state = state.into();
    Ok(())
}

fn process_update_default_account_state(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    state: AccountState,
) -> ProgramResult {
    check_valid_default_state(state)?;
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let freeze_authority_info = next_account_info(account_info_iter)?;
    let freeze_authority_info_data_len = freeze_authority_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;

    match &mint.base.freeze_authority {
        PodCOption {
            option: PodCOption::<Pubkey>::SOME,
            value: freeze_authority,
        } => Processor::validate_owner(
            program_id,
            freeze_authority,
            freeze_authority_info,
            freeze_authority_info_data_len,
            account_info_iter.as_slice(),
        ),
        _ => Err(TokenError::NoAuthorityExists.into()),
    }?;

    let extension = mint.get_extension_mut::<DefaultAccountState>()?;
    extension.state = state.into();
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    let (instruction, state) = decode_instruction(input)?;
    match instruction {
        DefaultAccountStateInstruction::Initialize => {
            msg!("DefaultAccountStateInstruction::Initialize");
            process_initialize_default_account_state(accounts, state)
        }
        DefaultAccountStateInstruction::Update => {
            msg!("DefaultAccountStateInstruction::Update");
            process_update_default_account_state(program_id, accounts, state)
        }
    }
}
