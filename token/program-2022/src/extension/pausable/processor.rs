use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            pausable::{
                instruction::{InitializeInstructionData, PausableInstruction},
                PausableConfig,
            },
            BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::PodMint,
        processor::Processor,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
};

fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    let extension = mint.init_extension::<PausableConfig>(true)?;
    extension.authority = Some(*authority).try_into()?;

    Ok(())
}

/// Pause or resume minting / burning / transferring on the mint
fn process_toggle_pause(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    pause: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<PausableConfig>()?;
    let maybe_authority: Option<Pubkey> = extension.authority.into();
    let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;

    Processor::validate_owner(
        program_id,
        &authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    extension.paused = pause.into();
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        PausableInstruction::Initialize => {
            msg!("PausableInstruction::Initialize");
            let InitializeInstructionData { authority } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority)
        }
        PausableInstruction::Pause => {
            msg!("PausableInstruction::Pause");
            process_toggle_pause(program_id, accounts, true /* pause */)
        }
        PausableInstruction::Resume => {
            msg!("PausableInstruction::Resume");
            process_toggle_pause(program_id, accounts, false /* resume */)
        }
    }
}
