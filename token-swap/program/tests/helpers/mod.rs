#![allow(dead_code)]

use {
    solana_program::{
        hash::Hash, instruction::Instruction, program_pack::Pack, pubkey::Pubkey,
        system_instruction,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        transport::TransportError,
    },
    spl_token::instruction::AuthorityType,
    spl_token_swap::{
        curve::{
            base::{CurveType, SwapCurve},
            constant_product::ConstantProductCurve,
            fees::Fees,
        },
        id, instruction, processor,
        state::PoolRegistry,
    },
    std::{
        convert::TryInto,
        sync::Arc,
    },
};

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_token_swap",
        id(),
        processor!(processor::Processor::process),
    )
}

pub async fn create_standard_setup<'a>(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    pool_registry_key: Option<Pubkey>,
    token_a_mint: &'a Keypair,
    token_b_mint: &'a Keypair,
    token_a_amount: u64,
    token_b_amount: u64,
) -> TokenSwapAccounts<'a> {
    let pool_registry_key = match pool_registry_key {
        Some(a) => a,
        None => create_pool_registry(banks_client, payer, recent_blockhash, payer)
            .await
            .unwrap(),
    };

    let fees = Fees {
        trade_fee_numerator: 20,
        trade_fee_denominator: 10000,
        owner_trade_fee_numerator: 10,
        owner_trade_fee_denominator: 10000,
        owner_withdraw_fee_numerator: 3,
        owner_withdraw_fee_denominator: 1000,
    };

    let swap_curve = SwapCurve {
        curve_type: CurveType::ConstantProduct,
        calculator: Arc::new(ConstantProductCurve {}),
    };

    let swap = TokenSwapAccounts::new(
        banks_client,
        payer,
        recent_blockhash,
        pool_registry_key,
        fees,
        swap_curve,
        token_a_mint,
        token_b_mint,
        token_a_amount,
        token_b_amount,
    )
    .await;

    swap
}

pub async fn create_depositor(
    banks_client: &mut BanksClient,
    mint_authority: &Keypair,
    recent_blockhash: &Hash,
    depositor: &Keypair,
    token_account_a: &Keypair,
    token_account_b: &Keypair,
    token_account_pool: &Keypair,
    token_a_mint_key: &Pubkey,
    token_b_mint_key: &Pubkey,
    token_pool_mint_key: &Pubkey,
    initial_a: u64,
    intiial_b: u64,
) {
    //token a
    create_token_account(
        banks_client,
        depositor,
        recent_blockhash,
        token_account_a,
        token_a_mint_key,
        &depositor.pubkey(),
    )
    .await
    .unwrap();
    mint_tokens(
        banks_client,
        depositor,
        recent_blockhash,
        token_a_mint_key,
        &token_account_a.pubkey(),
        mint_authority,
        initial_a,
    )
    .await
    .unwrap();

    //token b
    create_token_account(
        banks_client,
        depositor,
        recent_blockhash,
        token_account_b,
        token_b_mint_key,
        &depositor.pubkey(),
    )
    .await
    .unwrap();
    mint_tokens(
        banks_client,
        depositor,
        recent_blockhash,
        token_b_mint_key,
        &token_account_b.pubkey(),
        mint_authority,
        intiial_b,
    )
    .await
    .unwrap();

    //token pool
    create_token_account(
        banks_client,
        depositor,
        recent_blockhash,
        token_account_pool,
        token_pool_mint_key,
        &depositor.pubkey(),
    )
    .await
    .unwrap();
}

pub async fn create_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    owner: &Pubkey,
    len: u64,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_amt = rent.minimum_balance(len.try_into().unwrap());

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &account.pubkey(),
            rent_amt,
            0,
            owner,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub async fn create_account_with_seed(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    base: &Keypair,
    seed: &str,
    owner: &Pubkey,
    len: u64,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_amt = rent.minimum_balance(len.try_into().unwrap());

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account_with_seed(
            &payer.pubkey(),
            &account,
            &base.pubkey(),
            seed,
            rent_amt,
            len,
            owner,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, base], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Keypair,
    manager: &Pubkey,
    freeze_auth: Option<&Pubkey>,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                manager,
                freeze_auth,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_associated_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey, TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[
            spl_associated_token_account::create_associated_token_account(
                &payer.pubkey(),
                owner,
                mint,
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    Ok(spl_associated_token_account::get_associated_token_address(
        owner, mint,
    ))
}

pub async fn close_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    manager: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::close_account(
            &spl_token::id(),
            &account,
            &manager.pubkey(),
            &manager.pubkey(),
            &[&manager.pubkey()],
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, manager], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn burn_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::burn(
            &spl_token::id(),
            account,
            mint,
            &authority.pubkey(),
            &[&authority.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn change_token_owner(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Pubkey,
    current_owner: &Keypair,
    new_owner: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::set_authority(
            &spl_token::id(),
            account,
            Some(new_owner),
            AuthorityType::AccountOwner,
            &current_owner.pubkey(),
            &[&current_owner.pubkey()],
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, current_owner],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_pool_registry(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    pool_registry_payer: &Keypair,
) -> Result<Pubkey, TransportError> {
    let pool_registry_seed = "poolregistry";
    let pool_registry_key =
        Pubkey::create_with_seed(&payer.pubkey(), &pool_registry_seed, &id()).unwrap();

    let size = std::mem::size_of::<PoolRegistry>().try_into().unwrap();

    create_account_with_seed(
        banks_client,
        payer,
        recent_blockhash,
        &pool_registry_key,
        pool_registry_payer,
        "poolregistry",
        &id(),
        size,
    )
    .await
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_registry(
            &id(),
            &pool_registry_payer.pubkey(),
            &pool_registry_key,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, pool_registry_payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;

    Ok(pool_registry_key)
}

pub struct TokenSwapAccounts<'a> {
    pub fees: Fees,
    pub swap_curve: SwapCurve,
    pub swap_pubkey: Pubkey,
    pub authority_pubkey: Pubkey,
    pub nonce: u8,
    pub token_a_key: Keypair,
    pub token_a_mint_key: &'a Keypair,
    pub token_b_key: Keypair,
    pub token_b_mint_key: &'a Keypair,
    pub pool_mint_key: Keypair,
    pub pool_fee_key: Keypair,
    pub pool_fee_pubkey_override: Option<Pubkey>,
    pub pool_token_key: Keypair,
    pub pool_registry_pubkey: Pubkey,
    pub pool_nonce: u8,
}

impl<'a> TokenSwapAccounts<'a> {
    pub async fn new(
        mut banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        pool_registry_pubkey: Pubkey,
        fees: Fees,
        swap_curve: SwapCurve,
        token_a_mint_key: &'a Keypair,
        token_b_mint_key: &'a Keypair,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> TokenSwapAccounts<'a> {
        //random keys for these
        let token_a_key = Keypair::new();
        let token_b_key = Keypair::new();
        let pool_mint_key = Keypair::new();
        let pool_fee_key = Keypair::new();
        let pool_token_key = Keypair::new();

        let mut seed_key_vec = Vec::new();
        seed_key_vec.push(token_a_mint_key.pubkey().to_bytes());
        seed_key_vec.push(token_b_mint_key.pubkey().to_bytes());
        seed_key_vec.sort();

        //pda for swap account
        let (swap_pubkey, pool_nonce) = Pubkey::find_program_address(
            &[
                &seed_key_vec[0][..32],
                &seed_key_vec[1][..32],
                &[swap_curve.curve_type as u8],
            ],
            &id(),
        );

        //create the pda for the authority
        let (authority_pubkey, nonce) =
            Pubkey::find_program_address(&[&swap_pubkey.to_bytes()[..]], &spl_token_swap::id());

        //create the pool mint
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_mint_key,
            &authority_pubkey,
            None,
        )
        .await
        .unwrap();

        //create the pool fee account
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_fee_key,
            &pool_mint_key.pubkey(),
            &payer.pubkey(),
        )
        .await
        .unwrap();

        //create the pool token account
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_token_key,
            &pool_mint_key.pubkey(),
            &payer.pubkey(),
        )
        .await
        .unwrap();

        //create the A mint and pool's token account
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_mint_key,
            &payer.pubkey(),
            None,
        )
        .await
        .unwrap();
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_key,
            &token_a_mint_key.pubkey(),
            &authority_pubkey,
        )
        .await
        .unwrap();
        mint_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_mint_key.pubkey(),
            &token_a_key.pubkey(),
            &payer,
            token_a_amount,
        )
        .await
        .unwrap();

        //create the B mint and pool's token account
        create_mint(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_mint_key,
            &payer.pubkey(),
            None,
        )
        .await
        .unwrap();
        create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_key,
            &token_b_mint_key.pubkey(),
            &authority_pubkey,
        )
        .await
        .unwrap();
        mint_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_mint_key.pubkey(),
            &token_b_key.pubkey(),
            &payer,
            token_b_amount,
        )
        .await
        .unwrap();
        TokenSwapAccounts {
            fees,
            swap_curve,
            swap_pubkey,
            authority_pubkey,
            nonce,
            token_a_key,
            token_a_mint_key,
            token_b_key,
            token_b_mint_key,
            pool_mint_key,
            pool_fee_key,
            pool_fee_pubkey_override: None,
            pool_token_key,
            pool_registry_pubkey,
            pool_nonce,
        }
    }

    pub async fn initialize_swap(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
    ) -> Result<(), TransportError> {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::initialize(
                &id(),
                &spl_token::id(),
                &payer.pubkey(),
                &self.swap_pubkey,
                &self.authority_pubkey,
                &self.token_a_key.pubkey(),
                &self.token_b_key.pubkey(),
                &self.pool_mint_key.pubkey(),
                &self.pool_fee_key.pubkey(),
                &self.pool_token_key.pubkey(),
                self.nonce,
                self.fees.clone(),
                self.swap_curve.clone(),
                &self.pool_registry_pubkey,
                self.pool_nonce,
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await?;
        Ok(())
    }

    pub async fn routed_swap(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        other_swap: &TokenSwapAccounts<'a>,
        token_a: &Pubkey,
        token_b: Option<&Pubkey>,
        token_c: Option<&Pubkey>,
        amt_in: u64,
        amt_out: u64,
    ) -> Result<(), TransportError> {
        let mut ins = Vec::<Instruction>::new();
        //if token_b needs created, create it
        let token_b = match token_b {
            Some(t) => *t,
            _ => {
                //create ata
                ins.push(
                    spl_associated_token_account::create_associated_token_account(
                        &payer.pubkey(),
                        &payer.pubkey(),
                        &self.token_b_mint_key.pubkey(),
                    ),
                );
                spl_associated_token_account::get_associated_token_address(
                    &payer.pubkey(),
                    &self.token_b_mint_key.pubkey(),
                )
            }
        };
        //if token_c needs created, create it
        let token_c = match token_c {
            Some(t) => *t,
            _ => {
                //create ata
                ins.push(
                    spl_associated_token_account::create_associated_token_account(
                        &payer.pubkey(),
                        &payer.pubkey(),
                        &other_swap.token_b_mint_key.pubkey(),
                    ),
                );
                spl_associated_token_account::get_associated_token_address(
                    &payer.pubkey(),
                    &other_swap.token_b_mint_key.pubkey(),
                )
            }
        };

        //swap ins
        ins.push(
            instruction::routed_swap(
                &id(),
                &spl_token::id(),
                &self.swap_pubkey,
                &self.authority_pubkey,
                &payer.pubkey(),
                token_a,
                &self.token_a_key.pubkey(),
                &self.token_b_key.pubkey(),
                &token_b,
                &self.pool_mint_key.pubkey(),
                //allow fee key to change externally to a pubkey
                &self
                    .pool_fee_pubkey_override
                    .unwrap_or(self.pool_fee_key.pubkey()),
                &other_swap.swap_pubkey,
                &other_swap.authority_pubkey,
                &other_swap.token_a_key.pubkey(),
                &other_swap.token_b_key.pubkey(),
                &token_c,
                &other_swap.pool_mint_key.pubkey(),
                &other_swap.pool_fee_key.pubkey(),
                &payer.pubkey(),
                instruction::Swap {
                    amount_in: amt_in,
                    minimum_amount_out: amt_out,
                    flags: instruction::swap_flags::default_routed(),
                },
            )
            .unwrap(),
        );

        //now create and execute tx

        let mut transaction =
            Transaction::new_with_payer(&ins.into_boxed_slice(), Some(&payer.pubkey()));
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit_all_token_types(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        depositor_key: &Keypair,
        depositor_token_a_key: &Pubkey,
        depositor_token_b_key: &Pubkey,
        depositor_pool_key: &Pubkey,
        pool_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
    ) -> Result<(), TransportError> {
        let user_transfer_authority = Keypair::new();

        let mut transaction = Transaction::new_with_payer(
            &[
                spl_token::instruction::approve(
                    &spl_token::id(),
                    depositor_token_a_key,
                    &user_transfer_authority.pubkey(),
                    &depositor_key.pubkey(),
                    &[&depositor_key.pubkey()],
                    maximum_token_a_amount,
                )
                .unwrap(),
                spl_token::instruction::approve(
                    &spl_token::id(),
                    depositor_token_b_key,
                    &user_transfer_authority.pubkey(),
                    &depositor_key.pubkey(),
                    &[&depositor_key.pubkey()],
                    maximum_token_b_amount,
                )
                .unwrap(),
                instruction::deposit_all_token_types(
                    &id(),
                    &spl_token::id(),
                    &self.swap_pubkey,
                    &self.authority_pubkey,
                    &user_transfer_authority.pubkey(),
                    depositor_token_a_key,
                    depositor_token_b_key,
                    &self.token_a_key.pubkey(),
                    &self.token_b_key.pubkey(),
                    &self.pool_mint_key.pubkey(),
                    depositor_pool_key,
                    instruction::DepositAllTokenTypes {
                        pool_token_amount,
                        maximum_token_a_amount,
                        maximum_token_b_amount,
                    },
                )
                .unwrap(),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(
            &[payer, &user_transfer_authority, &depositor_key],
            *recent_blockhash,
        );
        banks_client.process_transaction(transaction).await?;
        Ok(())
    }

    pub async fn repair(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        ata: &Pubkey,
    ) -> Result<(), TransportError> {
        self.repair_override_old_fee(banks_client, payer, recent_blockhash, ata, None)
            .await
    }

    pub async fn repair_override_old_fee(
        &mut self,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        ata: &Pubkey,
        old_fee_account: Option<Pubkey>,
    ) -> Result<(), TransportError> {
        let mut transaction = Transaction::new_with_payer(
            &[instruction::repair_closed_fee_account(
                &id(),
                &self.swap_pubkey,
                &old_fee_account.unwrap_or_else(|| self.pool_fee_key.pubkey()),
                ata,
            )
            .unwrap()],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], *recent_blockhash);
        banks_client.process_transaction(transaction).await?;

        //future swaps should use this new key
        self.pool_fee_pubkey_override = Some(*ata);

        Ok(())
    }
}
