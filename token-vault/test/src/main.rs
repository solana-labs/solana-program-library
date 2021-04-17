use {
    clap::{crate_description, crate_name, crate_version, App, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_url, is_valid_pubkey, is_valid_signer},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{borsh::try_from_slice_unchecked, program_pack::Pack},
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction::create_account,
        transaction::Transaction,
    },
    spl_token::{
        instruction::{approve, initialize_account, initialize_mint, mint_to},
        state::{Account, Mint},
    },
    spl_token_vault::{
        instruction::{
            create_activate_vault_instruction, create_add_shares_instruction,
            create_add_token_to_inactive_vault_instruction, create_combine_vault_instruction,
            create_init_vault_instruction, create_mint_shares_instruction,
            create_redeem_shares_instruction, create_update_external_price_account_instruction,
            create_withdraw_shares_instruction, create_withdraw_tokens_instruction,
        },
        state::{
            ExternalPriceAccount, SafetyDepositBox, Vault, VaultState, MAX_EXTERNAL_ACCOUNT_SIZE,
            MAX_VAULT_SIZE, PREFIX,
        },
    },
    std::str::FromStr,
};

const PROGRAM_PUBKEY: &str = "94wRaYAQdC2gYF76AUTYSugNJ3rAC4EimjAMPwM7uYry";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

fn initialize_vault(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let vault_authority =
        pubkey_of(app_matches, "vault_authority").unwrap_or_else(|| payer.pubkey());
    let external_key = pubkey_of(app_matches, "external_price_account").unwrap();
    let external_account = client.get_account(&external_key).unwrap();
    let external: ExternalPriceAccount = try_from_slice_unchecked(&external_account.data).unwrap();
    let fraction_mint = Keypair::new();
    let redeem_mint = external.price_mint;
    let redeem_treasury = Keypair::new();
    let fraction_treasury = Keypair::new();
    let vault = Keypair::new();
    let allow_further_share_creation = app_matches.is_present("allow_further_share_creation");

    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let instructions = [
        create_account(
            &payer.pubkey(),
            &fraction_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &redeem_treasury.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &fraction_treasury.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &vault.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(MAX_VAULT_SIZE)
                .unwrap(),
            MAX_VAULT_SIZE as u64,
            &program_key,
        ),
        initialize_mint(
            &token_key,
            &fraction_mint.pubkey(),
            &authority,
            Some(&authority),
            0,
        )
        .unwrap(),
        initialize_account(
            &token_key,
            &redeem_treasury.pubkey(),
            &redeem_mint,
            &authority,
        )
        .unwrap(),
        initialize_account(
            &token_key,
            &fraction_treasury.pubkey(),
            &fraction_mint.pubkey(),
            &authority,
        )
        .unwrap(),
        create_init_vault_instruction(
            program_key,
            fraction_mint.pubkey(),
            redeem_treasury.pubkey(),
            fraction_treasury.pubkey(),
            vault.pubkey(),
            vault_authority,
            external_key,
            allow_further_share_creation,
        ),
    ];
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &redeem_treasury,
        &fraction_treasury,
        &fraction_mint,
        &vault,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _account = client.get_account(&vault.pubkey()).unwrap();
    vault.pubkey()
}

fn rewrite_price_account(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let external_account =
        read_keypair_file(app_matches.value_of("external_price_account").unwrap()).unwrap();
    let price_per_share: u64 = app_matches
        .value_of("price_per_share")
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap();
    let allowed_to_combine = app_matches.is_present("allowed_to_combine");
    let already_created = app_matches.is_present("already_created");
    let mut signers = vec![&payer, &external_account];

    let mut instructions = vec![];

    let key = Keypair::new();
    let price_mint = match pubkey_of(app_matches, "price_mint") {
        Some(val) => val,
        None => {
            // We make an empty oustanding share account if one is not provided.
            instructions.push(create_account(
                &payer.pubkey(),
                &key.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Mint::LEN)
                    .unwrap(),
                Mint::LEN as u64,
                &token_key,
            ));
            instructions.push(
                initialize_mint(
                    &token_key,
                    &key.pubkey(),
                    &payer.pubkey(),
                    Some(&payer.pubkey()),
                    0,
                )
                .unwrap(),
            );

            signers.push(&key);
            key.pubkey()
        }
    };

    if !already_created {
        instructions.push(create_account(
            &payer.pubkey(),
            &external_account.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(MAX_EXTERNAL_ACCOUNT_SIZE)
                .unwrap(),
            MAX_EXTERNAL_ACCOUNT_SIZE as u64,
            &program_key,
        ));
    }

    instructions.push(create_update_external_price_account_instruction(
        program_key,
        external_account.pubkey(),
        price_per_share,
        price_mint,
        allowed_to_combine,
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _account = client.get_account(&external_account.pubkey()).unwrap();
    external_account.pubkey()
}

#[allow(clippy::clone_on_copy)]
fn add_token_to_vault(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();
    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let amount: u64 = app_matches
        .value_of("amount")
        .unwrap_or("1")
        .parse::<u64>()
        .unwrap();

    let token_mint = Keypair::new();
    let token_account = Keypair::new();
    let store = Keypair::new();

    let transfer_authority = Keypair::new();

    let clone_of_key = token_mint.pubkey().clone();
    let seeds = &[
        PREFIX.as_bytes(),
        &vault_key.as_ref(),
        &clone_of_key.as_ref(),
    ];
    let (safety_deposit_box, _) = Pubkey::find_program_address(seeds, &program_key);
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let instructions = [
        create_account(
            &payer.pubkey(),
            &token_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &token_account.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &store.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        initialize_mint(
            &token_key,
            &token_mint.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            0,
        )
        .unwrap(),
        initialize_account(
            &token_key,
            &token_account.pubkey(),
            &token_mint.pubkey(),
            &payer.pubkey(),
        )
        .unwrap(),
        initialize_account(
            &token_key,
            &store.pubkey(),
            &token_mint.pubkey(),
            &authority,
        )
        .unwrap(),
        mint_to(
            &token_key,
            &token_mint.pubkey(),
            &token_account.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount,
        )
        .unwrap(),
        approve(
            &token_key,
            &token_account.pubkey(),
            &transfer_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount,
        )
        .unwrap(),
        create_add_token_to_inactive_vault_instruction(
            program_key,
            safety_deposit_box,
            token_account.pubkey(),
            store.pubkey(),
            vault_key,
            vault_authority.pubkey(),
            payer.pubkey(),
            transfer_authority.pubkey(),
            amount,
        ),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &token_mint,
        &token_account,
        &store,
        &vault_authority,
        &transfer_authority,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _account = client.get_account(&safety_deposit_box).unwrap();
    safety_deposit_box
}

fn activate_vault(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Option<Pubkey> {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();
    let number_of_shares: u64 = app_matches
        .value_of("number_of_shares")
        .unwrap_or("100")
        .parse::<u64>()
        .unwrap();
    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let vault_account = client.get_account(&vault_key).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();

    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (mint_authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let instructions = [create_activate_vault_instruction(
        program_key,
        vault_key,
        vault.fraction_mint,
        vault.fraction_treasury,
        mint_authority,
        vault_authority.pubkey(),
        number_of_shares,
    )];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&payer, &vault_authority];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let updated_vault_data = client.get_account(&vault_key).unwrap();
    let updated_vault: Vault = try_from_slice_unchecked(&updated_vault_data.data).unwrap();
    if updated_vault.state == VaultState::Active {
        println!("Activated vault.");
        Some(vault_key)
    } else {
        println!("Failed to update vault.");
        None
    }
}

fn combine_vault(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Option<Pubkey> {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let new_vault_authority =
        pubkey_of(app_matches, "new_vault_authority").unwrap_or_else(|| payer.pubkey());

    let amount_of_money: u64 = app_matches
        .value_of("amount_of_money")
        .unwrap_or("10000")
        .parse::<u64>()
        .unwrap();

    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let vault_account = client.get_account(&vault_key).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();
    let external_price_account = client.get_account(&vault.pricing_lookup_address).unwrap();
    let external: ExternalPriceAccount =
        try_from_slice_unchecked(&external_price_account.data).unwrap();
    let payment_account = Keypair::new();

    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (uncirculated_burn_authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let transfer_authority = Keypair::new();
    let mut signers = vec![
        &payer,
        &vault_authority,
        &payment_account,
        &transfer_authority,
    ];

    let mut instructions = vec![
        create_account(
            &payer.pubkey(),
            &payment_account.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        initialize_account(
            &token_key,
            &payment_account.pubkey(),
            &external.price_mint,
            &payer.pubkey(),
        )
        .unwrap(),
        mint_to(
            &token_key,
            &external.price_mint,
            &payment_account.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount_of_money,
        )
        .unwrap(),
        approve(
            &token_key,
            &payment_account.pubkey(),
            &transfer_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount_of_money,
        )
        .unwrap(),
    ];

    let mut shares_outstanding: u64 = 0;
    let key = Keypair::new();
    let outstanding_shares_account = match pubkey_of(app_matches, "outstanding_shares_account") {
        Some(val) => {
            let info = client.get_account(&val).unwrap();
            let account: Account = Account::unpack_unchecked(&info.data).unwrap();
            shares_outstanding = account.amount;
            val
        }
        None => {
            // We make an empty oustanding share account if one is not provided.
            instructions.push(create_account(
                &payer.pubkey(),
                &key.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Account::LEN)
                    .unwrap(),
                Account::LEN as u64,
                &token_key,
            ));
            instructions.push(
                initialize_account(
                    &token_key,
                    &key.pubkey(),
                    &vault.fraction_mint,
                    &payer.pubkey(),
                )
                .unwrap(),
            );

            signers.push(&key);
            key.pubkey()
        }
    };

    instructions.push(
        approve(
            &token_key,
            &outstanding_shares_account,
            &transfer_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            shares_outstanding,
        )
        .unwrap(),
    );

    instructions.push(create_combine_vault_instruction(
        program_key,
        vault_key,
        outstanding_shares_account,
        payment_account.pubkey(),
        vault.fraction_mint,
        vault.fraction_treasury,
        vault.redeem_treasury,
        new_vault_authority,
        vault_authority.pubkey(),
        transfer_authority.pubkey(),
        uncirculated_burn_authority,
        vault.pricing_lookup_address,
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let updated_vault_data = client.get_account(&vault_key).unwrap();
    let updated_vault: Vault = try_from_slice_unchecked(&updated_vault_data.data).unwrap();
    if updated_vault.state == VaultState::Combined {
        println!("Combined vault.");
        Some(vault_key)
    } else {
        println!("Failed to combined vault.");
        None
    }
}

fn redeem_shares(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let outstanding_shares_key = pubkey_of(app_matches, "outstanding_shares_account").unwrap();
    let outstanding_shares_account = client.get_account(&outstanding_shares_key).unwrap();
    let outstanding_shares: Account =
        Account::unpack_unchecked(&outstanding_shares_account.data).unwrap();
    let vault_account = client.get_account(&vault_key).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();
    let redeem_treasury_info = client.get_account(&vault.redeem_treasury).unwrap();
    let redeem_treasury: Account = Account::unpack_unchecked(&redeem_treasury_info.data).unwrap();

    let burn_authority = Keypair::new();
    let mut signers = vec![&payer, &vault_authority, &burn_authority];

    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (transfer_authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let mut instructions = vec![];

    let key = Keypair::new();
    let proceeds_account: Pubkey = match pubkey_of(app_matches, "proceeds_account") {
        Some(val) => val,
        None => {
            instructions.push(create_account(
                &payer.pubkey(),
                &key.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Account::LEN)
                    .unwrap(),
                Account::LEN as u64,
                &token_key,
            ));
            instructions.push(
                initialize_account(
                    &token_key,
                    &key.pubkey(),
                    &redeem_treasury.mint,
                    &payer.pubkey(),
                )
                .unwrap(),
            );
            signers.push(&key);
            key.pubkey()
        }
    };

    instructions.push(
        approve(
            &token_key,
            &outstanding_shares_key,
            &burn_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            outstanding_shares.amount,
        )
        .unwrap(),
    );

    instructions.push(create_redeem_shares_instruction(
        program_key,
        outstanding_shares_key,
        proceeds_account,
        vault.fraction_mint,
        vault.redeem_treasury,
        transfer_authority,
        burn_authority.pubkey(),
        vault_key,
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _new_proceeds = client.get_account(&proceeds_account).unwrap();
    proceeds_account
}

fn withdraw_tokens(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let safety_deposit_key = pubkey_of(app_matches, "safety_deposit_address").unwrap();
    let safety_deposit_account = client.get_account(&safety_deposit_key).unwrap();
    let safety_deposit: SafetyDepositBox =
        try_from_slice_unchecked(&safety_deposit_account.data).unwrap();
    let store_account = client.get_account(&safety_deposit.store).unwrap();
    let store: Account = Account::unpack_unchecked(&store_account.data).unwrap();
    let vault_account = client.get_account(&safety_deposit.vault).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();
    let amount: u64 = app_matches
        .value_of("amount")
        .unwrap_or(&store.amount.to_string())
        .parse::<u64>()
        .unwrap();

    let mut signers = vec![&payer, &vault_authority];
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (transfer_authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let mut instructions = vec![];

    let key = Keypair::new();
    let destination_account: Pubkey = match pubkey_of(app_matches, "destination_account") {
        Some(val) => val,
        None => {
            instructions.push(create_account(
                &payer.pubkey(),
                &key.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Account::LEN)
                    .unwrap(),
                Account::LEN as u64,
                &token_key,
            ));
            instructions.push(
                initialize_account(&token_key, &key.pubkey(), &store.mint, &payer.pubkey())
                    .unwrap(),
            );
            signers.push(&key);
            key.pubkey()
        }
    };

    instructions.push(create_withdraw_tokens_instruction(
        program_key,
        destination_account,
        safety_deposit_key,
        safety_deposit.store,
        safety_deposit.vault,
        vault.fraction_mint,
        vault_authority.pubkey(),
        transfer_authority,
        amount,
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _new_proceeds = client.get_account(&destination_account).unwrap();
    destination_account
}

fn mint_shares(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let vault_account = client.get_account(&vault_key).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();

    let signers = vec![&payer, &vault_authority];
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (mint_authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let number_of_shares: u64 = app_matches
        .value_of("number_of_shares")
        .unwrap_or("100")
        .parse::<u64>()
        .unwrap();

    let instructions = [create_mint_shares_instruction(
        program_key,
        vault.fraction_treasury,
        vault.fraction_mint,
        vault_key,
        mint_authority,
        vault_authority.pubkey(),
        number_of_shares,
    )];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _new_proceeds = client.get_account(&vault.fraction_treasury).unwrap();
    vault.fraction_treasury
}

fn withdraw_shares(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let vault_account = client.get_account(&vault_key).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();
    let number_of_shares: u64 = app_matches
        .value_of("number_of_shares")
        .unwrap_or("100")
        .parse::<u64>()
        .unwrap();

    let mut signers = vec![&payer, &vault_authority];
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref()];
    let (transfer_authority, _) = Pubkey::find_program_address(seeds, &program_key);

    let mut instructions = vec![];

    let key = Keypair::new();
    let destination_account: Pubkey = match pubkey_of(app_matches, "destination_account") {
        Some(val) => val,
        None => {
            instructions.push(create_account(
                &payer.pubkey(),
                &key.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Account::LEN)
                    .unwrap(),
                Account::LEN as u64,
                &token_key,
            ));
            instructions.push(
                initialize_account(
                    &token_key,
                    &key.pubkey(),
                    &vault.fraction_mint,
                    &payer.pubkey(),
                )
                .unwrap(),
            );
            signers.push(&key);
            key.pubkey()
        }
    };

    instructions.push(create_withdraw_shares_instruction(
        program_key,
        destination_account,
        vault.fraction_treasury,
        vault_key,
        transfer_authority,
        vault_authority.pubkey(),
        number_of_shares,
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _new_proceeds = client.get_account(&destination_account).unwrap();
    destination_account
}

fn add_shares(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    let vault_authority = read_keypair_file(
        app_matches
            .value_of("vault_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let vault_key = pubkey_of(app_matches, "vault_address").unwrap();
    let vault_account = client.get_account(&vault_key).unwrap();
    let vault: Vault = try_from_slice_unchecked(&vault_account.data).unwrap();
    let number_of_shares: u64 = app_matches
        .value_of("number_of_shares")
        .unwrap_or("100")
        .parse::<u64>()
        .unwrap();

    let transfer_authority = Keypair::new();
    let signers = [&payer, &vault_authority, &transfer_authority];

    let source_account: Pubkey = pubkey_of(app_matches, "source").unwrap();

    let instructions = [
        approve(
            &token_key,
            &source_account,
            &transfer_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            number_of_shares,
        )
        .unwrap(),
        create_add_shares_instruction(
            program_key,
            source_account,
            vault.fraction_treasury,
            vault_key,
            transfer_authority.pubkey(),
            vault_authority.pubkey(),
            number_of_shares,
        ),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    vault.fraction_treasury
}

fn main() {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::with_name("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster [default: devnet]"),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize a Vault")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of authority, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("external_price_account")
                        .long("external_price_account")
                        .value_name("EXTERNAL_PRICE_ACCOUNT")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of external price account"),
                )
                .arg(
                    Arg::with_name("allow_further_share_creation")
                        .long("allow_further_share_creation")
                        .value_name("ALLOW_FURTHER_SHARE_CREATION")
                        .takes_value(false)
                        .required(false)
                        .help("Allows further share creation after activation of vault"),
                ),
        )
        .subcommand(
            SubCommand::with_name("external_price_account_rewrite")
                .about("Rewrite (or create) an External Price Account")
                .arg(
                    Arg::with_name("external_price_account")
                        .long("external_price_account")
                        .value_name("EXTERNAL_PRICE_ACCOUNT")
                        .required(true)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair"),
                )
                .arg(
                    Arg::with_name("price_mint")
                        .long("price_mint")
                        .value_name("PRICE_MINT")
                        .takes_value(true)
                        .validator(is_valid_pubkey)
                        .required(false)
                        .help("Price mint that price per share uses"),
                )
                .arg(
                    Arg::with_name("price_per_share")
                        .long("price_per_share")
                        .value_name("PRICE_PER_SHARE")
                        .takes_value(true)
                        .required(false)
                        .help("Price per share"),
                )
                .arg(
                    Arg::with_name("allowed_to_combine")
                        .long("allowed_to_combine")
                        .value_name("ALLOWED_TO_COMBINE")
                        .takes_value(false)
                        .required(false)
                        .help("Whether or not combination is allowed in the vault"),
                )
                .arg(
                    Arg::with_name("already_created")
                        .long("already_created")
                        .value_name("ALREADY_CREATED")
                        .takes_value(false)
                        .required(false)
                        .help("If we should skip creation because this account already exists"),
                ),
        )
        .subcommand(
            SubCommand::with_name("add_token_to_vault")
                .about("Add Token of X amount (default 1) to Inactive Vault")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("vault_address")
                        .long("vault_address")
                        .value_name("VAULT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of vault"),
                )
                .arg(
                    Arg::with_name("amount")
                        .long("amount")
                        .value_name("AMOUNT")
                        .required(false)
                        .takes_value(true)
                        .help("Amount of this new token type to add to the vault"),
                ),
        )
        .subcommand(
            SubCommand::with_name("activate_vault")
                .about("Activate Vault")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("vault_address")
                        .long("vault_address")
                        .value_name("VAULT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of vault"),
                )
                .arg(
                    Arg::with_name("number_of_shares")
                        .long("number_of_shares")
                        .value_name("NUMBER_OF_SHARES")
                        .required(false)
                        .takes_value(true)
                        .help("Initial number of shares to produce, defaults to 100"),
                ),
        )
        .subcommand(
            SubCommand::with_name("combine_vault")
                .about("Combine Vault")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("vault_address")
                        .long("vault_address")
                        .value_name("VAULT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of vault"),
                ).arg(
                    Arg::with_name("new_vault_authority")
                        .long("new_vault_authority")
                        .value_name("NEW_VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("New authority of the vault going forward, defaults to you"),
                ).arg(
                    Arg::with_name("outstanding_shares_account")
                        .long("outstanding_shares_account")
                        .value_name("OUSTANDING_SHARES_ACCOUNT")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of oustanding shares account, an empty will be made if not provided"),
                ).arg(
                    Arg::with_name("amount_of_money")
                        .long("amount_of_money")
                        .value_name("AMOUNT_OF_MONEY")
                        .required(false)
                        .takes_value(true)
                        .help("Initial amount of money to provide to pay for buy out, defaults to 10000. You need to provide enough for a buy out!"),
                ),
        )
        .subcommand(
            SubCommand::with_name("redeem_shares")
                .about("Redeem Shares from a Combined Vault as a Shareholder")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("vault_address")
                        .long("vault_address")
                        .value_name("VAULT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of vault"),
                ).arg(
                    Arg::with_name("outstanding_shares_account")
                        .long("outstanding_shares_account")
                        .value_name("OUSTANDING_SHARES_ACCOUNT")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of oustanding shares account"),
                ).arg(
                    Arg::with_name("proceeds_account")
                        .long("proceeds_account")
                        .value_name("PROCEEDS_ACCOUNT")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of proceeds account, an empty will be made if not provided"),
                )
            )
        .subcommand(
        SubCommand::with_name("withdraw_tokens")
                .about("Withdraw Tokens from an Inactive or Combined Vault Safety Deposit Box")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("safety_deposit_address")
                        .long("safety_deposit_address")
                        .value_name("SAFETY_DEPOSIT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of safety deposit box"),
                ).arg(
                    Arg::with_name("destination_account")
                        .long("destination_account")
                        .value_name("DESTINATION_ACCOUNT")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of destination shares account, an empty will be made if not provided"),
                ).arg(
                    Arg::with_name("amount")
                        .long("amount")
                        .value_name("AMOUNT")
                        .required(false)
                        .takes_value(true)
                        .help("Amount of tokens to remove, defaults to all"),
                ))
        .subcommand(
            SubCommand::with_name("mint_shares")
                .about("Mint new shares to the fractional vault treasury")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("vault_address")
                        .long("vault_address")
                        .value_name("VAULT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the vault"),
                )
                .arg(
                    Arg::with_name("number_of_shares")
                        .long("number_of_shares")
                        .value_name("NUMBER_OF_SHARES")
                        .required(false)
                        .takes_value(true)
                        .help("Initial number of shares to produce, defaults to 100"),
                ))
        .subcommand(
            SubCommand::with_name("withdraw_shares")
                .about("Withdraw shares from the fractional treasury")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("vault_address")
                        .long("vault_address")
                        .value_name("VAULT_ADDRESS")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the vault"),
                )
                .arg(
                    Arg::with_name("number_of_shares")
                        .long("number_of_shares")
                        .value_name("NUMBER_OF_SHARES")
                        .required(false)
                        .takes_value(true)
                        .help("Initial number of shares to produce, defaults to 100"),
                ).arg(
                    Arg::with_name("destination_account")
                        .long("destination_account")
                        .value_name("DESTINATION_ACCOUNT")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of destination shares account, an empty will be made if not provided"),
                )).subcommand(
                    SubCommand::with_name("add_shares")
                        .about("Add shares to the fractional treasury")
                        .arg(
                            Arg::with_name("vault_authority")
                                .long("vault_authority")
                                .value_name("VAULT_AUTHORITY")
                                .required(false)
                                .validator(is_valid_signer)
                                .takes_value(true)
                                .help("Filepath or URL to a keypair, defaults to you otherwise"),
                        )
                        .arg(
                            Arg::with_name("vault_address")
                                .long("vault_address")
                                .value_name("VAULT_ADDRESS")
                                .required(true)
                                .validator(is_valid_pubkey)
                                .takes_value(true)
                                .help("Pubkey of the vault"),
                        )
                        .arg(
                            Arg::with_name("number_of_shares")
                                .long("number_of_shares")
                                .value_name("NUMBER_OF_SHARES")
                                .required(false)
                                .takes_value(true)
                                .help("Initial number of shares to produce, defaults to 100"),
                        ).arg(
                            Arg::with_name("source")
                                .long("source")
                                .value_name("SOURCE_ACCOUNT")
                                .required(true)
                                .validator(is_valid_pubkey)
                                .takes_value(true)
                                .help("Pubkey of source shares account"),
                        ))
        .get_matches();

    let client = RpcClient::new(
        app_matches
            .value_of("json_rpc_url")
            .unwrap_or(&"https://devnet.solana.com".to_owned())
            .to_owned(),
    );

    let (sub_command, sub_matches) = app_matches.subcommand();

    let payer = read_keypair_file(app_matches.value_of("keypair").unwrap()).unwrap();

    match (sub_command, sub_matches) {
        ("init", Some(arg_matches)) => {
            println!(
                "Created vault with address {:?}",
                initialize_vault(arg_matches, payer, client)
            );
        }
        ("external_price_account_rewrite", Some(arg_matches)) => {
            println!(
                "Rewrote price account {:?}",
                rewrite_price_account(arg_matches, payer, client)
            );
        }
        ("add_token_to_vault", Some(arg_matches)) => {
            println!(
                "Added token to safety deposit account {:?} to vault {:?}",
                add_token_to_vault(arg_matches, payer, client),
                arg_matches.value_of("vault_address").unwrap()
            );
        }
        ("activate_vault", Some(arg_matches)) => {
            activate_vault(arg_matches, payer, client);
            println!("Completed command.");
        }
        ("combine_vault", Some(arg_matches)) => {
            combine_vault(arg_matches, payer, client);
            println!("Completed command.");
        }
        ("redeem_shares", Some(arg_matches)) => {
            println!(
                "Redeemed share(s) and put monies in account {:?}",
                redeem_shares(arg_matches, payer, client)
            );
        }
        ("withdraw_tokens", Some(arg_matches)) => {
            println!(
                "Withdrew token(s) to account {:?}",
                withdraw_tokens(arg_matches, payer, client)
            );
        }
        ("mint_shares", Some(arg_matches)) => {
            println!(
                "Minted share(s) to fractional treasury {:?}",
                mint_shares(arg_matches, payer, client)
            );
        }
        ("withdraw_shares", Some(arg_matches)) => {
            println!(
                "Withdrew share(s) to account {:?}",
                withdraw_shares(arg_matches, payer, client)
            );
        }
        ("add_shares", Some(arg_matches)) => {
            println!(
                "Added share(s) to fractional treasury account {:?}",
                add_shares(arg_matches, payer, client)
            );
        }
        _ => unreachable!(),
    }
}
