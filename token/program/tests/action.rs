use {
    solana_program_test::BanksClient,
    solana_sdk::{
        hash::Hash,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
        transport::TransportError,
    },
    spl_token::{
        id, instruction,
        state::{Account, Mint},
    },
};

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    pool_mint: &Keypair,
    manager: &Pubkey,
    decimals: u8,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &pool_mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &id(),
            ),
            instruction::initialize_mint(&id(), &pool_mint.pubkey(), manager, None, decimals)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, pool_mint],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(Account::LEN);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                Account::LEN as u64,
                &id(),
            ),
            instruction::initialize_account(&id(), &account.pubkey(), pool_mint, owner).unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_to(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::mint_to(&id(), mint, account, &mint_authority.pubkey(), &[], amount)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn transfer(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::transfer(&id(), source, destination, &authority.pubkey(), &[], amount)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, authority],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn burn(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: Hash,
    mint: &Pubkey,
    account: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::burn(&id(), account, mint, &authority.pubkey(), &[], amount).unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}
