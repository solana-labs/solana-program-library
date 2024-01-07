//! src/processor.rs
//! Program state processor
//! 
use std::convert::TryInto;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, program_pack::Pack, program_option::COption, sysvar::Sysvar, msg,
};
use spl_token::state::Mint as MintOld;
use spl_token_2022::{state::Mint as Mint2022, instruction::decode_instruction_type};

use crate::{get_wrapped_mint_authority, state::Backpointer, instruction::TokenWrapInstruction, get_wrapped_mint_authority_with_seed, get_wrapped_mint_authority_seeds, get_wrapped_mint_address, get_wrapped_mint_backpointer_address, get_wrapped_mint_backpointer_address_seeds, get_wrapped_mint_backpointer_address_with_seed, get_wrapped_mint_signer_seeds, get_wrapped_mint_address_with_seed, get_wrapped_mint_backpointer_address_signer_seeds};



/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = decode_instruction_type(input)?;
    msg !("instructionii {:?}", instruction);
    match instruction {
        TokenWrapInstruction::CreateMint => {
            process_create_mint(program_id, accounts, input)
        }
        TokenWrapInstruction::Wrap => {
            process_wrap(program_id, accounts, input)
        }
        TokenWrapInstruction::Unwrap => {
            process_unwrap(program_id, accounts, input)
        }
    }
}

/// Process 'CreateMint' instruction
fn process_create_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let funding_account = next_account_info(account_info_iter)?;
    let wrapped_mint_account = next_account_info(account_info_iter)?;
    let backpointer_account = next_account_info(account_info_iter)?;
    let unwrapped_mint_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let sysvar_rent_account = next_account_info(account_info_iter)?;

    let rent = Rent::get()?;

    
    
    assert_eq!(token_program.key, &spl_token::ID, "{}", ProgramError::IncorrectProgramId);
    assert!(funding_account.is_signer, "{}", ProgramError::MissingRequiredSignature);
    assert!(wrapped_mint_account.lamports() == 0, "{}", ProgramError::AccountAlreadyInitialized);
    assert_eq!(backpointer_account.owner, &solana_program::system_program::ID, "{}", ProgramError::IncorrectProgramId);
    assert_eq!(backpointer_account.key, &get_wrapped_mint_backpointer_address(unwrapped_mint_account.key), "{}", ProgramError::InvalidArgument);
    assert_eq!(wrapped_mint_account.key, &get_wrapped_mint_address(unwrapped_mint_account.key, token_program.key), "{}", ProgramError::InvalidArgument);
    match *token_program.key {
        spl_token::ID => {
            // Handle SPL Token logic
            let lamports = rent.minimum_balance(MintOld::LEN);
            msg!("wrapped_mint account {:?}", wrapped_mint_account);
            msg!("unwrapped_mint account {:?}", unwrapped_mint_account);
            msg!("funding account {:?}", funding_account);
            msg!("lamports {:?}", lamports);
            let (_, bump_seed) = get_wrapped_mint_address_with_seed(unwrapped_mint_account.key, token_program.key);
            let bumps = &[bump_seed];
            let wrapped_mint_authority_seeds: &[&[u8]]  = &get_wrapped_mint_signer_seeds(unwrapped_mint_account.key, token_program.key, bumps);

            
            let create_account_ix = system_instruction::create_account(
                funding_account.key,
                wrapped_mint_account.key,
                lamports,
                MintOld::LEN as u64,
                &spl_token::ID,
            );

            invoke_signed(
                &create_account_ix,
                &[
                    funding_account.clone(),
                    wrapped_mint_account.clone(),
                    system_program.clone(),
                ],
                &[wrapped_mint_authority_seeds],
            )?;
            let unwrapped_mint_unpacked = Mint2022::unpack(&unwrapped_mint_account.data.borrow())?;
            let freeze_authority = match unwrapped_mint_unpacked.freeze_authority {
                COption::Some(authority) => Some(authority),
                COption::None => None,
            };
            msg!("freeze_authority {:?}", freeze_authority);
            // Initialize the wrapped mint using SPL Token
            let init_mint_ix = spl_token::instruction::initialize_mint(
                &spl_token::ID,
                wrapped_mint_account.key,
                &get_wrapped_mint_authority(wrapped_mint_account.key),
                freeze_authority.as_ref(),
                unwrapped_mint_unpacked.decimals,
            )?;
            msg!("init_mint_ix {:?}", init_mint_ix);
            invoke_signed(
                &init_mint_ix,
                &[
                    wrapped_mint_account.clone(),
                    token_program.clone(),
                    sysvar_rent_account.clone(),
                ],
                &[wrapped_mint_authority_seeds],
            )?;
            msg!("init_mint_ix {:?}", init_mint_ix);
        },
        spl_token_2022::ID => {
            // Handle SPL Token 2022 logic
            let lamports = rent.minimum_balance(Mint2022::LEN);
            let create_account_ix = system_instruction::create_account(
                funding_account.key,
                wrapped_mint_account.key,
                lamports,
                Mint2022::LEN as u64,
                &spl_token_2022::ID,
            );
            invoke(
                &create_account_ix,
                &[
                    funding_account.clone(),
                    wrapped_mint_account.clone(),
                    system_program.clone(),
                ],
            )?;
            
            let unwrapped_mint_unpacked = MintOld::unpack(&unwrapped_mint_account.data.borrow())?;
            let freeze_authority = match unwrapped_mint_unpacked.freeze_authority {
                COption::Some(authority) => Some(authority),
                COption::None => None,
            };
        // Initialize the wrapped mint using SPL Token 2022
        let init_mint_ix = spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            wrapped_mint_account.key,
            &get_wrapped_mint_authority(wrapped_mint_account.key),
            freeze_authority.as_ref(),
            unwrapped_mint_unpacked.decimals,
        )?;
        invoke(
            &init_mint_ix,
            &[
                wrapped_mint_account.clone(),
                token_program.clone(),
            ],
        )?;
        },
        _ => return Err(ProgramError::InvalidAccountData),
    }


    // Create and Initialize Backpointer Account
    let backpointer_lamports = rent.minimum_balance(std::mem::size_of::<Backpointer>());
    let (_, bump) = get_wrapped_mint_backpointer_address_with_seed(unwrapped_mint_account.key);
    let bump = [bump];
    let signer_seeds: &[&[u8]] = &get_wrapped_mint_backpointer_address_signer_seeds(unwrapped_mint_account.key, &bump);
   
    let create_backpointer_account_ix = system_instruction::create_account(
        funding_account.key,
        backpointer_account.key,
        backpointer_lamports,
        std::mem::size_of::<Backpointer>() as u64,
        program_id,
    );
    msg!("create_backpointer_account_ix {:?}", create_backpointer_account_ix);
    invoke_signed(
        &create_backpointer_account_ix,
        &[
            funding_account.clone(),
            backpointer_account.clone(),
            system_program.clone(),
        ],
        &[signer_seeds],
    )?;

    // Initialize Backpointer Account
    let backpointer_data = Backpointer {
        unwrapped_mint: *unwrapped_mint_account.key,
    };
    let data = &mut backpointer_account.data.borrow_mut();
    let backpointer_data_bytes = bytemuck::bytes_of(&backpointer_data);
    
    for (i, byte) in backpointer_data_bytes.iter().enumerate() {
        data[i] = *byte;
    }
    Ok(())
}

/// Process 'Wrap' instruction
fn process_wrap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_source_account = next_account_info(account_info_iter)?;
    let escrow_account = next_account_info(account_info_iter)?;
    let unwrapped_mint = next_account_info(account_info_iter)?;
    let wrapped_mint = next_account_info(account_info_iter)?;
    let user_destination_account = next_account_info(account_info_iter)?;
    let escrow_mint_authority = next_account_info(account_info_iter)?;
    let unwrapped_token_program = next_account_info(account_info_iter)?;
    let wrapped_token_program = next_account_info(account_info_iter)?;
    msg!("unwrapped_token_program {:?}", unwrapped_token_program.key);
    msg!("wrapped_token_program {:?}", wrapped_token_program.key);

    let signer = next_account_info(account_info_iter)?;
    let escrow_unwrapped_account = spl_token_2022::state::Account::unpack(&escrow_account.data.borrow())?;

    assert_eq!(escrow_mint_authority.key, &get_wrapped_mint_authority(wrapped_mint.key), "{}", ProgramError::InvalidArgument);
    assert_eq!(wrapped_mint.key, &get_wrapped_mint_address(&escrow_unwrapped_account.mint, wrapped_token_program.key), "{}", ProgramError::InvalidArgument);
    
    assert_eq!(escrow_mint_authority.key, &get_wrapped_mint_authority(wrapped_mint.key), "{}", ProgramError::InvalidArgument);
    // Parse the amount to wrap from the input
    let amount = unpack_amount(input[1..9].try_into().unwrap())?;
    
    
    match *wrapped_token_program.key {
        spl_token::ID => {
            assert_eq!(escrow_account.owner, &spl_token_2022::ID, "{}", ProgramError::IncorrectProgramId);
            assert_eq!(escrow_unwrapped_account.amount, amount, "{}", ProgramError::InsufficientFunds);
            // Handle wrapping logic for the original SPL Token program
            msg!("1");

            // Transfer unwrapped tokens to the escrow account
            let wrapped_mint_authority_seeds = get_wrapped_mint_authority_seeds(wrapped_mint.key);
            let (_, bump_seed) = get_wrapped_mint_authority_with_seed(wrapped_mint.key);
            let signer_seeds = &[
                &wrapped_mint_authority_seeds[0][..],
                &wrapped_mint_authority_seeds[1][..],
                &[bump_seed],
            ];

            let mint_to_user_account_ix = spl_token::instruction::mint_to(
                &spl_token::ID,
                wrapped_mint.key,
                user_destination_account.key,
                signer.key,
                &[],
                amount,
            )?;
            invoke_signed(
                &mint_to_user_account_ix,
                &[
                    wrapped_mint.clone(),
                    user_destination_account.clone(),
                    wrapped_token_program.clone(),
                    signer.clone(),
                ],
                &[signer_seeds],
            )?;

        },
        spl_token_2022::ID => {
            // Handle wrapping logic for SPL Token 2022 program
            // Fetch the decimals from the unwrapped token mint

            assert_eq!(escrow_account.owner, &spl_token::ID, "{}", ProgramError::IncorrectProgramId);
            msg!("2");
            assert_eq!(escrow_unwrapped_account.amount, amount, "{}", ProgramError::InsufficientFunds);

            // Transfer unwrapped tokens to the escrow account
            let wrapped_mint_authority_seeds = get_wrapped_mint_authority_seeds(wrapped_mint.key);
            let (_, bump_seed) = get_wrapped_mint_authority_with_seed(wrapped_mint.key);
            let signer_seeds = &[
                &wrapped_mint_authority_seeds[0][..],
                &wrapped_mint_authority_seeds[1][..],
                &[bump_seed],
            ];
            let unpacked_mint = Mint2022::unpack(&wrapped_mint.data.borrow())?;
            let mint_to_user_account_ix = spl_token_2022::instruction::mint_to_checked(
                &spl_token_2022::ID,
                wrapped_mint.key,
                user_destination_account.key,
                 signer.key,
                &[],
                amount,
                unpacked_mint.decimals,
            )?;
            invoke_signed(
                &mint_to_user_account_ix,
                &[
                    wrapped_mint.clone(),
                    user_destination_account.clone(),
                    wrapped_token_program.clone(),
                ],
                &[signer_seeds],
            )?;
        },
        _ => {
            // Handle unknown or unsupported token program
            return Err(ProgramError::InvalidAccountData);
        },
    }

    Ok(())
}
/// Process 'Unwrap' instruction
fn process_unwrap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    msg!("123");
    // Unwrap the required accounts from the account_info_iter
    let wrapped_token_account = next_account_info(account_info_iter)?;
    let escrow_account = next_account_info(account_info_iter)?;
    let wrapped_token_mint_account = next_account_info(account_info_iter)?;
    let user_unwrapped_token_account = next_account_info(account_info_iter)?;
    let burn_authority_account = next_account_info(account_info_iter)?;
    let token_program_account = next_account_info(account_info_iter)?;

    // Parse the amount to unwrap from the input
    let amount = unpack_amount(input[1..9].try_into().unwrap())?;
    
    // Derive the seeds and bump for the wrapped mint authority
    let (wrapped_mint_authority, bump_seed) = get_wrapped_mint_authority_with_seed(wrapped_token_mint_account.key);
    let wrapped_mint_authority_seeds = get_wrapped_mint_authority_seeds(wrapped_token_mint_account.key);
    let signer_seeds = &[
        &wrapped_mint_authority_seeds[0][..],
        &wrapped_mint_authority_seeds[1][..],
        &[bump_seed],
    ];

    
    assert!(wrapped_token_account.is_signer, "{}", ProgramError::MissingRequiredSignature);
    assert_eq!(wrapped_token_account.owner, token_program_account.key, "{}", ProgramError::IncorrectProgramId);
    assert!(amount > 0, "{}", ProgramError::InvalidArgument);
    assert!(escrow_account.lamports() >= amount, "{}", ProgramError::InsufficientFunds);
    assert_eq!(wrapped_token_mint_account.owner, token_program_account.key, "{}", ProgramError::IncorrectProgramId);
    assert_eq!(user_unwrapped_token_account.owner, token_program_account.key, "{}", ProgramError::IncorrectProgramId);
    assert!(burn_authority_account.is_signer, "{}", ProgramError::MissingRequiredSignature);


    match *token_program_account.key {
        spl_token::ID => {
            // Burn wrapped tokens from the user's account
            let burn_wrapped_tokens_ix = spl_token::instruction::burn(
                &spl_token::ID,
                wrapped_token_account.key,
                wrapped_token_mint_account.key,
                burn_authority_account.key,
                &[],
                amount,
            )?;
            invoke(
                &burn_wrapped_tokens_ix,
                &[
                    wrapped_token_account.clone(),
                    wrapped_token_mint_account.clone(),
                    burn_authority_account.clone(),
                    token_program_account.clone(),
                ],
            )?;

            // Transfer unwrapped tokens from the escrow to the user's account using invoke_signed
            let transfer_unwrapped_tokens_ix = spl_token::instruction::transfer(
                &spl_token::ID,
                escrow_account.key,
                user_unwrapped_token_account.key,
                &wrapped_mint_authority,
                &[],
                amount,
            )?;
            invoke_signed(
                &transfer_unwrapped_tokens_ix,
                &[
                    escrow_account.clone(),
                    user_unwrapped_token_account.clone(),
                    token_program_account.clone(),
                ],
                &[signer_seeds],
            )?;
        },
        spl_token_2022::ID => {
            // Ensure the token program is SPL Token 2022
            if *token_program_account.key != spl_token_2022::ID {
                return Err(ProgramError::IncorrectProgramId);
            }

            // Fetch the decimals from the wrapped token mint
            let wrapped_mint_info = Mint2022::unpack(&wrapped_token_mint_account.data.borrow())?;
            let decimals = wrapped_mint_info.decimals;

            // Burn wrapped tokens from the user's account using burn_checked
            let burn_wrapped_tokens_ix = spl_token_2022::instruction::burn_checked(
                &spl_token_2022::ID,
                wrapped_token_account.key,
                wrapped_token_mint_account.key,
                burn_authority_account.key,
                &[],
                amount,
                decimals,
            )?;
            invoke(
                &burn_wrapped_tokens_ix,
                &[
                    wrapped_token_account.clone(),
                    wrapped_token_mint_account.clone(),
                    burn_authority_account.clone(),
                    token_program_account.clone(),
                ],
            )?;

            // Transfer unwrapped tokens from the escrow to the user's account using invoke_signed and transfer_checked
            let transfer_unwrapped_tokens_ix = spl_token_2022::instruction::transfer_checked(
                &spl_token_2022::ID,
                escrow_account.key,
                wrapped_token_mint_account.key,
                user_unwrapped_token_account.key,
                &wrapped_mint_authority,
                &[],
                amount,
                decimals,
            )?;
            invoke_signed(
                &transfer_unwrapped_tokens_ix,
                &[
                    escrow_account.clone(),
                    user_unwrapped_token_account.clone(),
                    token_program_account.clone(),
                ],
                &[signer_seeds],
            )?;
        },
        _ => {
            // Handle unknown or unsupported token program
            return Err(ProgramError::InvalidAccountData);
        },
    }
    Ok(())
}

// Helper function to unpack the amount from the instruction input
fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
    assert_eq!(input.len(), 8, "{}", ProgramError::InvalidInstructionData);
    Ok(u64::from_le_bytes(input.try_into().unwrap()))
}

#[cfg(test)]
mod tests {

    use crate::{get_wrapped_mint_backpointer_address, get_wrapped_mint_address, instruction};

    use super::*;
    use borsh::Serializable;
    use solana_program::{hash::Hash, system_program, instruction::{Instruction, AccountMeta}, sysvar::recent_blockhashes};
    use solana_program_test::*;
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use spl_token::
        instruction as token_instruction;
    use spl_token_2022::instruction as token_instruction_2022;

    // Helper function to create a test environment
    async fn setup() -> (BanksClient, Keypair, Hash) {
        let program_id = crate::ID;
        let mut program_test = ProgramTest::new(
            "spl_token_wrap", // The name of your program
            program_id,       // The ID of your program
            processor!(process_instruction), // The entrypoint of your programs
        );

        // Add the SPL Token program to the test environment
        program_test.add_program(
            "spl_token",
            spl_token::ID,
            processor!(spl_token::processor::Processor::process),
        );

        // Add the SPL Token 2022 program to the test environment

        program_test.add_program(
            "spl_token_2022",
            spl_token_2022::ID,
            processor!(spl_token_2022::processor::Processor::process),
        );

        // Start the test environment

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        // init payer

        let mut transaction = Transaction::new_with_payer(
            &[system_instruction::transfer(
                &payer.pubkey(),
                &payer.pubkey(),
                100_000_000,
            )],
            Some(&payer.pubkey()),
        );

        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

        transaction.sign(&[&payer], recent_blockhash);

        banks_client.process_transaction(transaction).await.unwrap();
        
        // Return the test environment

        (banks_client, payer, recent_blockhash)

    }
    // Test for process_create_mint
    #[tokio::test]
    async fn test_e2e() {
        let (mut banks_client, payer, recent_blockhash) = setup().await;

        
        println!("2");
        let unwrapped_mint_keypair = Keypair::new();
        // Define test mint
        let mint_keypair = get_wrapped_mint_address(&unwrapped_mint_keypair.pubkey(), &spl_token::ID);
        let mut transaction = Transaction::new_with_payer(
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &unwrapped_mint_keypair.pubkey(),
                    Rent::default().minimum_balance(Mint2022::LEN),
                    Mint2022::LEN as u64,
                    &spl_token_2022::ID,
                ),
                token_instruction_2022::initialize_mint(
                &spl_token_2022::ID,
                &unwrapped_mint_keypair.pubkey(),
                &payer.pubkey(),
                None,
                2,
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        println!("1");
        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
        transaction.sign(&[&payer, &unwrapped_mint_keypair], recent_blockhash);
        println!("2");
        banks_client.process_transaction(transaction).await.unwrap();

        let backpointer_account = get_wrapped_mint_backpointer_address(&unwrapped_mint_keypair.pubkey());
        let system_program = system_program::ID;
        
        let accounts = vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(mint_keypair, false),
            AccountMeta::new(backpointer_account, false),
            AccountMeta::new_readonly(unwrapped_mint_keypair.pubkey(), false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false),
        ];

        let mut instruction_data = TokenWrapInstruction::CreateMint.try_to_vec().unwrap();

        instruction_data.extend_from_slice(&[0]);
        let instruction = Instruction {
            program_id: crate::ID,
            accounts,
            data: instruction_data,
        };

        let mut transaction = Transaction::new_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
        );
        let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

        transaction.sign(&[&payer], recent_blockhash);

        println!("3");
        banks_client.process_transaction(transaction).await.unwrap();
        // Assertions to ensure the mint creation was successful
        let mint_account = banks_client.get_account(mint_keypair).await.unwrap().unwrap();
        assert!(mint_account.data.len() > 0); // Ensure the mint account data is initialized

        let user_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[
        system_instruction::create_account(
            &payer.pubkey(),
            &user_account.pubkey(),
            Rent::default().minimum_balance(spl_token_2022::state::Account::LEN),
            spl_token_2022::state::Account::LEN as u64,
            &spl_token_2022::ID,
        ),
        token_instruction_2022::initialize_account(
            &spl_token_2022::ID,
            &user_account.pubkey(),
            &unwrapped_mint_keypair.pubkey(),
            &payer.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    println!("4");
    transaction.sign(&[&payer, &user_account], recent_blockhash);

    banks_client.process_transaction(transaction).await.unwrap();

    // Transfer some tokens to the user's source token account

    let mint_amount = 1000_000_000; // 100 tokens (assuming 2 decimals)

    let mut transaction = Transaction::new_with_payer(
        &[token_instruction_2022::mint_to_checked(
            &spl_token_2022::ID,
            &unwrapped_mint_keypair.pubkey(),
            &user_account.pubkey(),
            &payer.pubkey(),
            &[],
            mint_amount,
            2
        ).unwrap()],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    println!("5");
    transaction.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(transaction).await.unwrap();

    // Create and initialize the user's wrapped token account
    let user_wrapped_account = Keypair::new();
    println!("6");
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &user_wrapped_account.pubkey(),
                Rent::default().minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &spl_token::ID,
            ),
            token_instruction::initialize_account(
            &spl_token::ID,
            &user_wrapped_account.pubkey(),
            &mint_keypair,
            &payer.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    transaction.sign(&[&payer, &user_wrapped_account], recent_blockhash);

    println!("7");
    banks_client.process_transaction(transaction).await.unwrap();

    // Create and initialize the escrow account
    let escrow_account = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &escrow_account.pubkey(),
                Rent::default().minimum_balance(spl_token_2022::state::Account::LEN),
                spl_token_2022::state::Account::LEN as u64,
                &spl_token_2022::ID,
            ),
            token_instruction_2022::initialize_account(
                &spl_token_2022::ID,
                &escrow_account.pubkey(),
                &unwrapped_mint_keypair.pubkey(),
                &get_wrapped_mint_authority(&mint_keypair)).unwrap()],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    println!("8");

    transaction.sign(&[&payer, &escrow_account], recent_blockhash);
    
    banks_client.process_transaction(transaction).await.unwrap();
    // Define the amount to wrap
    let amount_to_wrap: u64 = 100_000_000; // 100 tokens (assuming 2 decimals)s
    // transfer 100_000_000 to escrow account
    let mut transaction = Transaction::new_with_payer(
        &[
        spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::ID,
            &user_account.pubkey(),
            &unwrapped_mint_keypair.pubkey(),
            &escrow_account.pubkey(),
            &payer.pubkey(),
            &[],
            amount_to_wrap as u64,
            2
        ).unwrap()],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    println!("9");
    transaction.sign(&[&payer], recent_blockhash);
    
    println!("10");
    banks_client.process_transaction(transaction).await.unwrap();
    let wrap_accounts = vec![
        AccountMeta::new(user_account.pubkey(), false),
        AccountMeta::new(escrow_account.pubkey(), false),
        AccountMeta::new_readonly(unwrapped_mint_keypair.pubkey(), false),
        AccountMeta::new(mint_keypair, false),
        AccountMeta::new(user_wrapped_account.pubkey(), false),
        AccountMeta::new(get_wrapped_mint_authority(&mint_keypair), false),
        AccountMeta::new_readonly(spl_token_2022::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(payer.pubkey(), true)
    ];

    let mut wrap_instruction_data = instruction::TokenWrapInstruction::Wrap.try_to_vec().unwrap();
    let amount: [u8; 8] = amount_to_wrap.to_le_bytes();
    wrap_instruction_data.extend_from_slice(&amount);
    let wrap_instruction = Instruction {
        program_id: crate::ID,
        accounts: wrap_accounts,
        data: wrap_instruction_data,
    };

    let mut transaction = Transaction::new_with_payer(
        &[wrap_instruction],
        Some(&payer.pubkey()),
    );
    let keypairs = [&payer];
    let signers = keypairs
    .iter()
    .map(|k| k as &dyn solana_sdk::signature::Signer) // Cast each Keypair as a Signer
    .collect::<Vec<_>>();

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    transaction.sign(
        &signers.clone(),
        recent_blockhash,
    );

    println!("9");
    banks_client.process_transaction(transaction).await.unwrap();

    // Assertions to ensure the wrap was successful

    // Check if the wrapped tokens were correctly minted to the destination account
    let user_wrapped_account_data = banks_client
        .get_account(user_wrapped_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let wrapped_token_account_state = spl_token::state::Account::unpack(&user_wrapped_account_data.data).unwrap();
    assert_eq!(wrapped_token_account_state.amount, amount_to_wrap as u64);

    // Check if the correct amount was deducted from the user source account
    let user_account_data = banks_client
        .get_account(user_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let user_token_account_state = spl_token::state::Account::unpack(&user_account_data.data).unwrap();
    assert_eq!(user_token_account_state.amount, 0);

    // Check if the escrow account holds the correct amount of unwrapped tokens
    let escrow_account_data = banks_client
        .get_account(escrow_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let escrow_token_account_state = spl_token::state::Account::unpack(&escrow_account_data.data).unwrap();
    assert_eq!(escrow_token_account_state.amount, amount_to_wrap as u64);
    }
// ... [previous test_process_create_mint code]


// ... [previous test_process_wrap code]

// Test for process_unwrap
#[tokio::test]
async fn test_process_unwrap() {
    let (mut banks_client, payer, recent_blockhash) = setup().await;

    // Define test accounts and other necessary variables here...
    let user_wrapped_account = Keypair::new(); // User's account for wrapped tokens
    let escrow_account = Keypair::new(); // Escrow account holding unwrapped tokens
    let user_account = Keypair::new(); // User's account to receive unwrapped tokens
    let wrapped_mint_account = Keypair::new(); // Wrapped mint account

    // Initialize accounts and simulate wrapping tokens first
    // ...

    // Define the amount to unwrap
    let amount_to_unwrap: u64 = 100_000_000; // 100 tokens (assuming 2 decimals)

    // Create and process the transaction for unwrapping tokens
    let unwrap_accounts = vec![
        AccountMeta::new(user_wrapped_account.pubkey(), true),
        AccountMeta::new(escrow_account.pubkey(), false),
        AccountMeta::new(wrapped_mint_account.pubkey(), false),
        AccountMeta::new(user_account.pubkey(), false),
        AccountMeta::new_readonly(spl_token::ID, false),
        // Add other necessary accounts...
    ];

    let mut unwrap_instruction_data = instruction::TokenWrapInstruction::Unwrap.try_to_vec().unwrap();
    unwrap_instruction_data.extend_from_slice(&amount_to_unwrap.to_le_bytes());

    let unwrap_instruction = Instruction {
        program_id: crate::ID,
        accounts: unwrap_accounts,
        data: unwrap_instruction_data,
    };

    let mut transaction = Transaction::new_with_payer(
        &[unwrap_instruction],
        Some(&payer.pubkey()),
    );
    let keypairs = [&payer, &user_wrapped_account, &escrow_account, &wrapped_mint_account, &user_account];
    let signers = keypairs
    .iter()
    .map(|k| k as &dyn solana_sdk::signature::Signer) // Cast each Keypair as a Signer
    .collect::<Vec<_>>();


    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    transaction.sign(
        &signers,
        recent_blockhash,
    );
    println!("10");
    banks_client.process_transaction(transaction).await.unwrap();

    // Assertions to ensure the unwrap was successful

    // Check if the unwrapped tokens were correctly credited to the user's account
    let user_account_data = banks_client
        .get_account(user_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let user_token_account_state = spl_token::state::Account::unpack(&user_account_data.data).unwrap();
    assert_eq!(user_token_account_state.amount, amount_to_unwrap as u64);

    // Check if the correct amount was deducted from the wrapped tokens account
    let user_wrapped_account_data = banks_client
        .get_account(user_wrapped_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let wrapped_token_account_state = spl_token::state::Account::unpack(&user_wrapped_account_data.data).unwrap();
    assert_eq!(wrapped_token_account_state.amount, 0);

    // Check if the escrow account released the correct amount of unwrapped tokens
    let escrow_account_data = banks_client
        .get_account(escrow_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let escrow_token_account_state = spl_token::state::Account::unpack(&escrow_account_data.data).unwrap();
    assert_eq!(escrow_token_account_state.amount, 0); // Assuming all tokens were unwrapped
}

    }