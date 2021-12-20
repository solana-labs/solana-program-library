mod utils;
mod vault_actions;

#[test]
#[ignore]
fn test_vault_usdc_usdt() {
    vault_actions::run_test(
        "SBR.STC.USDC-USDT",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "USDC",
            amount: 0.222,
        }],
        vec![utils::Swap {
            protocol: "SBR",
            from_token: "USDT",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_usdc_wust_v1() {
    vault_actions::run_test(
        "SBR.STC.USDC-WUST_V1",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "USDC",
            amount: 0.221,
        }],
        vec![utils::Swap {
            protocol: "SBR",
            from_token: "WUST_V1",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_acusd_usdc() {
    vault_actions::run_test(
        "SBR.STC.ACUSD-USDC",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "USDC",
            amount: 0.223,
        }],
        vec![utils::Swap {
            protocol: "SBR",
            from_token: "ACUSD",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_wdai_usdc() {
    vault_actions::run_test(
        "SBR.STC.WDAI-USDC",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "USDC",
            amount: 0.224,
        }],
        vec![utils::Swap {
            protocol: "SBR",
            from_token: "WDAI",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}
