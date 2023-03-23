use {
    crate::instruction::{PadInstruction, WrapData},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::convert::TryInto,
};

pub fn process(
    _program_id: &Pubkey,
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, rest) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    match (*tag)
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        PadInstruction::Noop => Ok(()),
        PadInstruction::Wrap => {
            let WrapData {
                num_accounts,
                instruction_size,
                instruction_data,
            } = WrapData::unpack(rest)?;
            let mut data = Vec::with_capacity(instruction_size as usize);
            data.extend_from_slice(instruction_data);

            let program_id = *account_infos[num_accounts as usize].key;

            let accounts = account_infos
                .iter()
                .take(num_accounts as usize)
                .map(|a| AccountMeta {
                    pubkey: *a.key,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect::<Vec<_>>();

            let instruction = Instruction {
                program_id,
                accounts,
                data,
            };

            invoke(&instruction, &account_infos[..num_accounts as usize])
        }
    }
}
