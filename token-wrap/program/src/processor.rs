//! Program state processor

use solana_program::program::{invoke, invoke_signed};

use crate::{instruction::{wrap, transfer, mint_to, create_mint, burn}, get_wrapped_mint_backpointer_address, get_wrapped_mint_authority_with_seed, get_wrapped_mint_address};

use {
    crate::instruction::TokenWrapInstruction,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey},
    spl_token_2022::instruction::decode_instruction_type,
};

/// Instruction processor
pub fn process_instruction(
    _program_account: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    match decode_instruction_type(input)? {
        TokenWrapInstruction::CreateMint => {
            let program_account = &accounts[0];
            let mint_account = &accounts[1];
            let mint_authority_account = &accounts[2];
            let freeze_authority_account = &accounts[3];
            let token_program_account = &accounts[4];
            let wrapped_mint_account = &accounts[5];
            let unwrapped_mint_account = &accounts[6];
            let system_program_account = &accounts[7];
            let wrapped_token_program_account = &accounts[8];
            let backpointer_pubkey = get_wrapped_mint_backpointer_address(wrapped_mint_account.key);

            assert!(mint_account.key == &get_wrapped_mint_address(unwrapped_mint_account.key,
                wrapped_token_program_account.key));

            

            let idempotent: bool = input[1] == 1;
            //program_id, funder_pubkey, wrapped_mint_pubkey, backpointer_pubkey, unwrapped_mint_pubkey, system_program_id, wrapped_token_program_id, idempotent)
            let ix = create_mint(
                program_account.key,
                program_account.key,
                mint_account.key,
                &backpointer_pubkey,
                unwrapped_mint_account.key,
                system_program_account.key,
                wrapped_token_program_account.key,
                idempotent,
            )?;
            invoke(&ix, &[
                mint_account.clone(),
                mint_authority_account.clone(),
                freeze_authority_account.clone(),
                token_program_account.clone(),
            ])?;
            Ok(())
        }
        TokenWrapInstruction::Wrap => {
            let program_account = &accounts[0];
            let unwrapped_token_account_account = &accounts[1];
            let escrow_account = &accounts[2];
            let unwrapped_mint_account = &accounts[3];
            let wrapped_mint_account = &accounts[4];
            let recipient_account = &accounts[5];
            let escrow_authority_account = &accounts[6];
            let wrapped_token_program_account = &accounts[7];
            let transfer_authority_account = &accounts[8];
            let unwrapped_token_program_account: &AccountInfo<'_> = &accounts[9];
            let token_program_account = &accounts[10];
            
            
            assert!(wrapped_mint_account.key == &get_wrapped_mint_address(unwrapped_mint_account.key,
                wrapped_token_program_account.key));
            assert!(wrapped_mint_account.owner == wrapped_token_program_account.key);
            assert!(escrow_account.owner == &get_wrapped_mint_authority_with_seed(wrapped_mint_account.key).0);
            
            // ? breaking: fix this
            let amount: u64 = (&input[1..9]).into_iter().rev().fold(0, |acc, &x| (acc << 8) + x as u64);
            let ix1 = transfer(unwrapped_token_program_account.key,
                 unwrapped_token_account_account.key,
                  escrow_account.key,
                   transfer_authority_account.key,
                    amount)?;
            let ix2 = mint_to(wrapped_token_program_account.key, 
                wrapped_mint_account.key, 
                recipient_account.key, 
                transfer_authority_account.key, 
                amount)?;
            invoke(&ix1, 
                &[
                    unwrapped_token_account_account.clone(),
                    escrow_account.clone(),
                    transfer_authority_account.clone(),
                    unwrapped_token_program_account.clone(),
                
                ]
            )?;
            invoke(&ix2, 
                &[
                    wrapped_mint_account.clone(),
                    recipient_account.clone(),
                    transfer_authority_account.clone(),
                    token_program_account.clone(),
                
                ]
            )?;
            let ix3 = wrap(
                program_account.key,
                unwrapped_token_program_account.key,
                escrow_account.key,
                unwrapped_mint_account.key,
                wrapped_mint_account.key,
                recipient_account.key,
                escrow_authority_account.key,
                wrapped_token_program_account.key,
                transfer_authority_account.key,
                amount
            )?;
            invoke(&ix3, 
                &[
                    program_account.clone(),
                    unwrapped_token_account_account.clone(),
                    escrow_account.clone(),
                    unwrapped_mint_account.clone(),
                    wrapped_mint_account.clone(),
                    recipient_account.clone(),
                    escrow_authority_account.clone(),
                    wrapped_token_program_account.clone(),
                    transfer_authority_account.clone()
                
                ]
            )?;
            Ok(())
        }
        TokenWrapInstruction::Unwrap => {
                    
            let program_account = &accounts[0];
            let unwrapped_token_account_account = &accounts[1];
            let escrow_account = &accounts[2];
            let unwrapped_mint_account = &accounts[3];
            let wrapped_mint_account = &accounts[4];
            let recipient_account = &accounts[5];
            let escrow_authority_account = &accounts[6];
            let wrapped_token_program_account = &accounts[7];
            let transfer_authority_account = &accounts[8];
            let wrapped_token_account_account = &accounts[9];
            let token_program_account = &accounts[10];
            let unwrapped_token_program_account: &AccountInfo<'_> = &accounts[11];

            assert!(wrapped_mint_account.key == &get_wrapped_mint_address(unwrapped_mint_account.key,
                wrapped_token_program_account.key));
            assert!(wrapped_mint_account.owner == wrapped_token_program_account.key);
            assert!(escrow_account.owner == &get_wrapped_mint_authority_with_seed(wrapped_mint_account.key).0);

            // ? breaking: fix this
            let amount: u64 = (&input[1..9]).into_iter().rev().fold(0, |acc, &x| (acc << 8) + x as u64);
            //program_id, burn_account_pubkey, mint_pubkey, burn_authority_pubkey, amount)
            let ix1 = burn(
                wrapped_token_program_account.key,
                wrapped_token_account_account.key,
                wrapped_mint_account.key,
                escrow_authority_account.key,
                amount
            )?;
            let ix2 = transfer(
                unwrapped_token_program_account.key,
                escrow_account.key,
                unwrapped_token_account_account.key,
                escrow_authority_account.key,
                amount
            )?;
            let signer_seeds = get_wrapped_mint_authority_with_seed(wrapped_mint_account.key);
            invoke_signed(&ix1, 
                &[
                    wrapped_token_account_account.clone(),
                    wrapped_mint_account.clone(),
                    transfer_authority_account.clone(),
                    token_program_account.clone(),
                ],
                &[&[&[signer_seeds.1]]]
            )?;
            invoke_signed(&ix2, 
                &[
                    unwrapped_mint_account.clone(),
                    recipient_account.clone(),
                    transfer_authority_account.clone(),
                    token_program_account.clone(),
                ],
                &[&[&[signer_seeds.1]]]
            )?;
            let ix3 = wrap(
                program_account.key,
                unwrapped_token_program_account.key,
                escrow_account.key,
                unwrapped_mint_account.key,
                wrapped_mint_account.key,
                recipient_account.key,
                escrow_authority_account.key,
                wrapped_token_program_account.key,
                transfer_authority_account.key,
                amount
            )?;
            invoke(&ix3, 
                &[
                    program_account.clone(),
                    unwrapped_token_account_account.clone(),
                    escrow_account.clone(),
                    unwrapped_mint_account.clone(),
                    wrapped_mint_account.clone(),
                    recipient_account.clone(),
                    escrow_authority_account.clone(),
                    wrapped_token_program_account.clone(),
                    transfer_authority_account.clone()
                
                ]
            )?;
            Ok(())
        }
    }
}