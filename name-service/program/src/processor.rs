use {
    crate::{
        instruction::NameRegistryInstruction,
        state::get_seeds_and_key,
        state::{write_data, NameRecordHeader},
    },
    borsh::BorshDeserialize,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction,
    },
};

pub struct Processor {}

impl Processor {
    pub fn process_create(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        hashed_name: Vec<u8>,
        lamports: u64,
        space: u32,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let system_program = next_account_info(accounts_iter)?;
        let payer_account = next_account_info(accounts_iter)?;
        let name_account = next_account_info(accounts_iter)?;
        let name_owner = next_account_info(accounts_iter)?;
        let name_class = next_account_info(accounts_iter)?;
        let parent_name_account = next_account_info(accounts_iter)?;
        let parent_name_owner = next_account_info(accounts_iter).ok();

        let (name_account_key, seeds) = get_seeds_and_key(
            program_id,
            hashed_name,
            Some(name_class.key),
            Some(parent_name_account.key),
        );

        // Verifications
        if name_account_key != *name_account.key {
            msg!("The given name account is incorrect.");
            return Err(ProgramError::InvalidArgument);
        }
        if name_account.data.borrow().len() > 0 {
            let name_record_header =
                NameRecordHeader::unpack_from_slice(&name_account.data.borrow())?;
            if name_record_header.owner != Pubkey::default() {
                msg!("The given name account already exists.");
                return Err(ProgramError::InvalidArgument);
            }
        }
        if *name_class.key != Pubkey::default() && !name_class.is_signer {
            msg!("The given name class is not a signer.");
            return Err(ProgramError::InvalidArgument);
        }
        if *parent_name_account.key != Pubkey::default() {
            if !parent_name_owner.unwrap().is_signer {
                msg!("The given parent name account owner is not a signer.");
                return Err(ProgramError::InvalidArgument);
            } else {
                let parent_name_record_header =
                    NameRecordHeader::unpack_from_slice(&parent_name_account.data.borrow())?;
                if &parent_name_record_header.owner != parent_name_owner.unwrap().key {
                    msg!("The given parent name account owner is not correct.");
                    return Err(ProgramError::InvalidArgument);
                }
            }
        }
        if name_owner.key == &Pubkey::default() {
            msg!("The owner cannot be `Pubkey::default()`.");
            return Err(ProgramError::InvalidArgument);
        }

        if name_account.data.borrow().len() == 0 {
            // Issue the name registry account
            // The creation is done in three steps: transfer, allocate and assign, because
            // one cannot `system_instruction::create` an account to which lamports have been transfered before.
            invoke(
                &system_instruction::transfer(payer_account.key, &name_account_key, lamports),
                &[
                    payer_account.clone(),
                    name_account.clone(),
                    system_program.clone(),
                ],
            )?;

            invoke_signed(
                &system_instruction::allocate(
                    &name_account_key,
                    (NameRecordHeader::LEN + space as usize) as u64,
                ),
                &[name_account.clone(), system_program.clone()],
                &[&seeds.chunks(32).collect::<Vec<&[u8]>>()],
            )?;

            invoke_signed(
                &system_instruction::assign(name_account.key, program_id),
                &[name_account.clone(), system_program.clone()],
                &[&seeds.chunks(32).collect::<Vec<&[u8]>>()],
            )?;
        }

        let name_state = NameRecordHeader {
            parent_name: *parent_name_account.key,
            owner: *name_owner.key,
            class: *name_class.key,
        };

        name_state.pack_into_slice(&mut name_account.data.borrow_mut());

        Ok(())
    }

    pub fn process_update(accounts: &[AccountInfo], offset: u32, data: Vec<u8>) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let name_account = next_account_info(accounts_iter)?;
        let name_update_signer = next_account_info(accounts_iter)?;
        let parent_name = next_account_info(accounts_iter).ok();

        let name_record_header = NameRecordHeader::unpack_from_slice(&name_account.data.borrow())?;

        // Verifications
        let is_parent_owner = if let Some(parent_name) = parent_name {
            if name_record_header.parent_name != *parent_name.key {
                msg!("Invalid parent name account");
                return Err(ProgramError::InvalidArgument);
            }
            let parent_name_record_header =
                NameRecordHeader::unpack_from_slice(&parent_name.data.borrow())?;
            parent_name_record_header.owner == *name_update_signer.key
        } else {
            false
        };
        if !name_update_signer.is_signer {
            msg!("The given name class or owner is not a signer.");
            return Err(ProgramError::InvalidArgument);
        }
        if name_record_header.class != Pubkey::default()
            && *name_update_signer.key != name_record_header.class
        {
            msg!("The given name class account is incorrect.");
            return Err(ProgramError::InvalidArgument);
        }
        if name_record_header.class == Pubkey::default()
            && *name_update_signer.key != name_record_header.owner
            && !is_parent_owner
        {
            msg!("The given name owner account is incorrect.");
            return Err(ProgramError::InvalidArgument);
        }

        write_data(name_account, &data, NameRecordHeader::LEN + offset as usize);

        Ok(())
    }

    pub fn process_transfer(accounts: &[AccountInfo], new_owner: Pubkey) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let name_account = next_account_info(accounts_iter)?;
        let name_owner = next_account_info(accounts_iter)?;
        let name_class_opt = next_account_info(accounts_iter).ok();
        let parent_name = next_account_info(accounts_iter).ok();

        let mut name_record_header =
            NameRecordHeader::unpack_from_slice(&name_account.data.borrow())?;

        // Verifications
        let is_parent_owner = if let Some(parent_name) = parent_name {
            if name_record_header.parent_name != *parent_name.key {
                msg!("Invalid parent name account");
                return Err(ProgramError::InvalidArgument);
            }
            let parent_name_record_header =
                NameRecordHeader::unpack_from_slice(&parent_name.data.borrow())?;
            parent_name_record_header.owner == *name_owner.key
        } else {
            false
        };
        if !name_owner.is_signer
            || (name_record_header.owner != *name_owner.key && !is_parent_owner)
        {
            msg!("The given name owner is incorrect or not a signer.");
            return Err(ProgramError::InvalidArgument);
        }
        if name_record_header.class != Pubkey::default()
            && (name_class_opt.is_none()
                || name_record_header.class != *name_class_opt.unwrap().key
                || !name_class_opt.unwrap().is_signer)
        {
            msg!("The given name class account is incorrect or not a signer.");
            return Err(ProgramError::InvalidArgument);
        }

        name_record_header.owner = new_owner;
        name_record_header
            .pack_into_slice(&mut name_account.data.borrow_mut()[..NameRecordHeader::LEN]);

        Ok(())
    }

    pub fn process_delete(accounts: &[AccountInfo]) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let name_account = next_account_info(accounts_iter)?;
        let name_owner = next_account_info(accounts_iter)?;
        let refund_target = next_account_info(accounts_iter)?;

        let name_record_header = NameRecordHeader::unpack_from_slice(&name_account.data.borrow())?;

        // Verifications
        if !name_owner.is_signer || name_record_header.owner != *name_owner.key {
            msg!("The given name owner is incorrect or not a signer.");
            return Err(ProgramError::InvalidArgument);
        }

        // Overwrite the data with zeroes
        write_data(name_account, &vec![0; name_account.data_len()], 0);

        // Close the account by transferring the rent sol
        let source_amount: &mut u64 = &mut name_account.lamports.borrow_mut();
        let dest_amount: &mut u64 = &mut refund_target.lamports.borrow_mut();
        *dest_amount += *source_amount;
        *source_amount = 0;

        Ok(())
    }

    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        msg!("Beginning processing");
        let instruction = NameRegistryInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        msg!("Instruction unpacked");

        match instruction {
            NameRegistryInstruction::Create {
                hashed_name,
                lamports,
                space,
            } => {
                msg!("Instruction: Create");
                Processor::process_create(program_id, accounts, hashed_name, lamports, space)?;
            }
            NameRegistryInstruction::Update { offset, data } => {
                msg!("Instruction: Update Data");
                Processor::process_update(accounts, offset, data)?;
            }
            NameRegistryInstruction::Transfer { new_owner } => {
                msg!("Instruction: Transfer Ownership");
                Processor::process_transfer(accounts, new_owner)?;
            }
            NameRegistryInstruction::Delete => {
                msg!("Instruction: Delete Name");
                Processor::process_delete(accounts)?;
            }
        }
        Ok(())
    }
}
