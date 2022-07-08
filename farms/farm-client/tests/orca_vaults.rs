mod utils;
mod vault_actions;

#[test]
#[ignore]
fn test_vault_chicks_usdc() {
    vault_actions::run_test(
        "ORC.STC.CHICKS-USDC-AQ-V1",
        vec![
            utils::Swap {
                protocol: "ORC",
                from_token: "SOL",
                to_token: "USDC",
                amount: 1.222,
            },
            utils::Swap {
                protocol: "ORC",
                from_token: "USDC",
                to_token: "CHICKS",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "ORC",
            from_token: "CHICKS",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_chicks_usdc_latest() {
    vault_actions::run_test(
        "ORC.STC.CHICKS-USDC-AQ",
        vec![
            utils::Swap {
                protocol: "ORC",
                from_token: "SOL",
                to_token: "USDC",
                amount: 1.222,
            },
            utils::Swap {
                protocol: "ORC",
                from_token: "USDC",
                to_token: "CHICKS",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "ORC",
            from_token: "CHICKS",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}
