//! Handler for the governance commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        id::{
            main_router_admin, ProgramIDType, DAO_CUSTODY_NAME, DAO_MINT_NAME, DAO_PROGRAM_NAME,
            DAO_TOKEN_NAME,
        },
        refdb::StorageType,
        string::str_to_as64,
        token::{Token, TokenType},
    },
    solana_sdk::{program_pack::Pack, pubkey::Pubkey},
    spl_associated_token_account::{create_associated_token_account, get_associated_token_address},
    spl_governance::instruction as dao_instruction,
    spl_governance::state::{
        enums::{MintMaxVoteWeightSource, VoteThresholdPercentage, VoteWeightSource},
        governance::{get_account_governance_address, GovernanceConfig},
        realm::get_realm_address,
        token_owner_record::get_token_owner_record_address,
    },
};

pub fn init(client: &FarmClient, config: &Config, dao_program: &Pubkey, mint_ui_amount: f64) {
    info!("Initializing DAO...");

    let wallet = config.keypair.pubkey();
    if main_router_admin::id() != wallet {
        panic!(
            "DAO must be initialized with the admin account {}",
            main_router_admin::id()
        );
    }
    if mint_ui_amount < 100.0 {
        panic!("Mint amount must be >= 100");
    }

    let mut inst = vec![];

    info!("  Writing Program \"{}\" to on-chain RefDB...", dao_program);
    client
        .add_program_id(
            config.keypair.as_ref(),
            DAO_PROGRAM_NAME,
            dao_program,
            ProgramIDType::System,
            None,
        )
        .unwrap();

    let mint_address = Pubkey::create_with_seed(&wallet, DAO_MINT_NAME, &spl_token::id()).unwrap();
    let mint_size = spl_token::state::Mint::get_packed_len();
    let dao_token_address = get_associated_token_address(&wallet, &mint_address);

    if client.rpc_client.get_account_data(&mint_address).is_err() {
        info!(
            "  Creating governance tokens mint at {} and minting {} tokens...",
            mint_address, mint_ui_amount
        );

        // record token info to the refdb
        let (index, counter) = if let Ok(token) = client.get_token(DAO_TOKEN_NAME) {
            (token.refdb_index, token.refdb_counter)
        } else {
            (
                Some(
                    client
                        .get_refdb_last_index(&StorageType::Token.to_string())
                        .expect("Token RefDB query error"),
                ),
                0u16,
            )
        };
        let token = Token {
            name: str_to_as64(DAO_TOKEN_NAME).unwrap(),
            description: str_to_as64("Solana Farms Governance Token").unwrap(),
            token_type: TokenType::SplToken,
            refdb_index: index,
            refdb_counter: counter,
            decimals: 6,
            chain_id: 101,
            mint: mint_address,
        };

        inst.push(client.new_instruction_add_token(&wallet, token).unwrap());

        // initialize governance tokens mint
        inst.push(
            client
                .new_instruction_create_system_account_with_seed(
                    &wallet,
                    &wallet,
                    DAO_MINT_NAME,
                    0,
                    mint_size,
                    &spl_token::id(),
                )
                .unwrap(),
        );

        inst.push(
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_address,
                &wallet,
                Some(&wallet),
                6,
            )
            .unwrap(),
        );

        if client
            .rpc_client
            .get_account_data(&dao_token_address)
            .is_err()
        {
            inst.push(create_associated_token_account(
                &wallet,
                &wallet,
                &mint_address,
            ));
        }

        // mint governance tokens to admin account first
        inst.push(
            spl_token::instruction::mint_to(
                &spl_token::id(),
                &mint_address,
                &dao_token_address,
                &wallet,
                &[],
                client.ui_amount_to_tokens_with_decimals(mint_ui_amount, 6),
            )
            .unwrap(),
        );

        info!(
            "  Signature: {}",
            client
                .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
                .unwrap()
        );
    }

    info!("  Creating realm and depositing DAO tokens...");

    // create realm
    inst.clear();
    let realm_address = get_realm_address(dao_program, DAO_PROGRAM_NAME);

    if client.rpc_client.get_account_data(&realm_address).is_err() {
        inst.push(dao_instruction::create_realm(
            dao_program,
            &wallet,
            &mint_address,
            &wallet,
            None,
            None,
            DAO_PROGRAM_NAME.to_string(),
            client.ui_amount_to_tokens_with_decimals(1.0, 6),
            MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
        ));
    }

    // deposit governance tokens
    inst.push(dao_instruction::deposit_governing_tokens(
        dao_program,
        &realm_address,
        &dao_token_address,
        &wallet,
        &wallet,
        &wallet,
        client.ui_amount_to_tokens_with_decimals(1.0, 6),
        &mint_address,
    ));

    info!(
        "  Signature: {}",
        client
            .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
            .unwrap()
    );

    // create router program governances
    info!("  Creating router program governances...");
    inst.clear();
    let dao_config = GovernanceConfig {
        vote_threshold_percentage: VoteThresholdPercentage::YesVote(60),
        min_community_tokens_to_create_proposal: (mint_ui_amount as f64 * 0.01) as u64,
        min_instruction_hold_up_time: 0,
        max_voting_time: 259200,
        vote_weight_source: VoteWeightSource::Deposit,
        proposal_cool_off_time: 0,
        min_council_tokens_to_create_proposal: 0,
    };
    let token_owner =
        get_token_owner_record_address(dao_program, &realm_address, &mint_address, &wallet);
    for program_name in &[
        DAO_PROGRAM_NAME,
        "MainRouter",
        "RaydiumRouter",
        "SaberRouter",
        "OrcaRouter",
    ] {
        let program = if program_name == &DAO_PROGRAM_NAME {
            *dao_program
        } else {
            client.get_program_id(program_name).unwrap()
        };
        inst.push(dao_instruction::create_program_governance(
            dao_program,
            &realm_address,
            &program,
            &wallet,
            &token_owner,
            &wallet,
            &wallet,
            None,
            dao_config.clone(),
            true,
        ));
    }

    info!(
        "  Signature: {}",
        client
            .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
            .unwrap()
    );

    // create vault program governances
    info!("  Creating vault program governances...");
    inst.clear();
    let vaults = client.get_vaults().unwrap();
    for (_vault_name, vault) in vaults {
        inst.push(dao_instruction::create_program_governance(
            dao_program,
            &realm_address,
            &vault.vault_program_id,
            &wallet,
            &token_owner,
            &wallet,
            &wallet,
            None,
            dao_config.clone(),
            true,
        ));
    }

    info!(
        "  Signature: {}",
        client
            .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
            .unwrap()
    );

    // create DAO mint governance
    info!("  Creating DAO mint governance...");
    inst.clear();
    inst.push(dao_instruction::create_mint_governance(
        dao_program,
        &realm_address,
        &mint_address,
        &wallet,
        &token_owner,
        &wallet,
        &wallet,
        None,
        dao_config.clone(),
        true,
    ));

    info!(
        "  Signature: {}",
        client
            .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
            .unwrap()
    );

    // create token custody governance
    info!("  Creating token custody governance...");
    inst.clear();
    let governed_account =
        Pubkey::find_program_address(&[DAO_CUSTODY_NAME.as_bytes()], dao_program).0;
    let custody_authority =
        get_account_governance_address(dao_program, &realm_address, &governed_account);

    // create wsol account for custody authority
    if !client.has_active_token_account(&custody_authority, "SOL") {
        let wsol_token = client.get_token("SOL").unwrap();
        inst.push(create_associated_token_account(
            &wallet,
            &custody_authority,
            &wsol_token.mint,
        ));
    }

    inst.push(dao_instruction::create_account_governance(
        dao_program,
        &realm_address,
        &governed_account,
        &token_owner,
        &wallet,
        &wallet,
        None,
        dao_config,
    ));

    inst.push(
        client
            .new_instruction_transfer(&wallet, &custody_authority, 0.1)
            .unwrap(),
    );

    info!(
        "  Signature: {}",
        client
            .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
            .unwrap()
    );

    // remove realm authority
    info!("  Removing realm authority...");
    inst.clear();
    inst.push(dao_instruction::set_realm_authority(
        dao_program,
        &realm_address,
        &wallet,
        &None,
    ));

    info!(
        "  Signature: {}",
        client
            .sign_and_send_instructions(&[config.keypair.as_ref()], inst.as_slice())
            .unwrap()
    );

    info!("Done.");
}
