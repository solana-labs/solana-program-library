//! Creates a file that will set constants captured from the environment.
//! These constants represent official accounts, program ids, and names.
//! Normally lazy_static! would work, but it is not supported with build-bpf.

use {
    solana_program::pubkey::Pubkey,
    std::{env, fs::File, io::Write, path::Path, str::FromStr},
};

fn main() {
    // create output file
    let out_dir = env::var("OUT_DIR")
        .expect("Please set OUT_DIR environment variable to the build script output path");
    let dest_path = Path::new(&out_dir).join("constants.rs");
    let mut f = File::create(&dest_path).unwrap_or_else(|_| {
        panic!(
            "Could not create file \"{}\" for the build script output",
            dest_path.to_string_lossy()
        )
    });

    // read variables
    let main_router_id =
        option_env!("MAIN_ROUTER_ID").unwrap_or("RepLaceThisWithVaLidMainRouterProgramPubkey");
    if main_router_id == "RepLaceThisWithVaLidMainRouterProgramPubkey" {
        println!("cargo:warning=Please set MAIN_ROUTER_ID environment variable to the router-main program address");
    }

    let main_router_admin =
        option_env!("MAIN_ROUTER_ADMIN").unwrap_or("RepLaceThisWithCorrectMainRouterAdminPubkey");
    if main_router_admin == "RepLaceThisWithCorrectMainRouterAdminPubkey" {
        println!("cargo:warning=Please set MAIN_ROUTER_ADMIN environment variable to the router-main program admin");
    }

    let main_router_multisig =
        Pubkey::find_program_address(&[b"multisig"], &Pubkey::from_str(main_router_id).unwrap()).0;

    // ID that represents the unset Pubkey. This is to avoid passing Pubkey::default() which
    // is equal to system_program::id().
    // Default is [14, 196, 109, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    let zero_id = option_env!("ZERO_ID").unwrap_or("zeRosMEYuuABXv5y2LNUbgmPp62yFD5CULW5soHS9HR");

    let dao_token_name = option_env!("DAO_TOKEN_NAME").unwrap_or("FARM_DAO");
    let dao_program_name = option_env!("DAO_PROGRAM_NAME").unwrap_or("FarmGovernance");
    let dao_mint_name = option_env!("DAO_MINT_NAME").unwrap_or("FarmMintGovernance");
    let dao_custody_name = option_env!("DAO_CUSTODY_NAME").unwrap_or("FarmCustodyGovernance");

    // write the file
    let write_error = format!(
        "Could not write to the build script output file: {}",
        dest_path.to_string_lossy()
    );

    writeln!(
        &mut f,
        "pub mod main_router {{solana_program::declare_id!(\"{}\"); }}",
        main_router_id
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub mod main_router_admin {{solana_program::declare_id!(\"{}\"); }}",
        main_router_admin
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub mod main_router_multisig {{solana_program::declare_id!(\"{}\"); }}",
        main_router_multisig
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub mod zero {{solana_program::declare_id!(\"{}\"); }}",
        zero_id
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub const DAO_TOKEN_NAME: &str = \"{}\";",
        dao_token_name
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub const DAO_PROGRAM_NAME: &str = \"{}\";",
        dao_program_name
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub const DAO_MINT_NAME: &str = \"{}\";",
        dao_mint_name
    )
    .expect(&write_error);

    writeln!(
        &mut f,
        "pub const DAO_CUSTODY_NAME: &str = \"{}\";",
        dao_custody_name
    )
    .expect(&write_error);

    // specify when to re-create
    println!("cargo:rerun-if-env-changed=MAIN_ROUTER_ID");
    println!("cargo:rerun-if-env-changed=MAIN_ROUTER_ADMIN");
    println!("cargo:rerun-if-env-changed=ZERO_ID");
    println!("cargo:rerun-if-env-changed=DAO_TOKEN_NAME");
    println!("cargo:rerun-if-env-changed=DAO_PROGRAM_NAME");
    println!("cargo:rerun-if-env-changed=DAO_MINT_NAME");
    println!("cargo:rerun-if-env-changed=DAO_CUSTODY_NAME");
}
