mod utils;
mod vault_actions;

#[test]
#[ignore]
fn test_vault_polis_ray() {
    vault_actions::run_test(
        "RDM.STC.POLIS-RAY-V5",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.222,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "POLIS",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "POLIS",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_polis_ray_latest() {
    vault_actions::run_test(
        "RDM.STC.POLIS-RAY",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.222,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "POLIS",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "POLIS",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_sny_ray() {
    vault_actions::run_test(
        "RDM.STC.SNY-RAY-V5",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.222,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "SNY",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SNY",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_atlas_ray() {
    vault_actions::run_test(
        "RDM.STC.ATLAS-RAY-V5",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.222,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "ATLAS",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "ATLAS",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_ray_srm_v3() {
    vault_actions::run_test(
        "RDM.STC.RAY-SRM-V3",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.123,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "SRM",
                amount: 0.123,
            },
        ],
        vec![],
    );
}

#[test]
#[ignore]
fn test_vault_ray_srm_v5() {
    vault_actions::run_test(
        "RDM.STC.RAY-SRM-V5",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.123,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "SRM",
                amount: 0.123,
            },
        ],
        vec![],
    );
}

#[test]
#[ignore]
fn test_vault_ray_srm_latest() {
    vault_actions::run_test(
        "RDM.STC.RAY-SRM",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.123,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "SRM",
                amount: 0.123,
            },
        ],
        vec![],
    );
}

#[test]
#[ignore]
fn test_vault_grape_usdc() {
    vault_actions::run_test(
        "RDM.STC.GRAPE-USDC-V5",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "USDC",
                amount: 0.222,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "USDC",
                to_token: "GRAPE",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "GRAPE",
            to_token: "USDC",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_samo_ray() {
    vault_actions::run_test(
        "RDM.STC.SAMO-RAY-V5",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.211,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "SAMO",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SAMO",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_oxy_ray() {
    vault_actions::run_test(
        "RDM.STC.OXY-RAY-V4",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.233,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "OXY",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "OXY",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_oxy_ray_latest() {
    vault_actions::run_test(
        "RDM.STC.OXY-RAY",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.233,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "OXY",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "OXY",
            to_token: "RAY",
            amount: 0.0,
        }],
    );
}

#[test]
#[ignore]
fn test_vault_ray_sol() {
    vault_actions::run_test(
        "RDM.STC.RAY-SOL-V3",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "RAY",
            amount: 0.091111,
        }],
        vec![],
    );
}

#[test]
#[ignore]
fn test_vault_ray_sol_latest() {
    vault_actions::run_test(
        "RDM.STC.RAY-SOL",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "RAY",
            amount: 0.091111,
        }],
        vec![],
    );
}

#[test]
#[ignore]
fn test_vault_ray_usdt() {
    vault_actions::run_test(
        "RDM.STC.RAY-USDT-V3",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.223,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "USDT",
                amount: -0.5,
            },
        ],
        vec![],
    );
}

/*
#[test]
#[ignore]
fn all_vault_tests() {
    // dual v5
    test_vault_polis_ray();
    test_vault_polis_ray_latest();
    test_vault_atlas_ray();
    test_vault_sny_ray();
    test_vault_ray_srm_v5();
    test_vault_ray_srm_latest();

    // single reward b v5
    test_vault_grape_usdc();
    test_vault_samo_ray();

    // dual v4
    test_vault_oxy_ray();
    test_vault_oxy_ray_latest();

    // single reward a v3
    test_vault_ray_sol();
    test_vault_ray_sol_latest();
    test_vault_ray_usdt();
    test_vault_ray_srm_v3();
}*/
