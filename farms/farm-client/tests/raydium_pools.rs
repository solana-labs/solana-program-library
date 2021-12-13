mod pool_actions;
mod utils;

#[test]
#[ignore]
fn test_pool_ray_srm() {
    pool_actions::run_test(
        "RDM.RAY-SRM-V4",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.111,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "SRM",
                amount: 0.111,
            },
        ],
        vec![],
        false,
    );
}

#[test]
#[ignore]
fn test_pool_ray_srm_latest() {
    pool_actions::run_test(
        "RDM.RAY-SRM",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.111,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "SRM",
                amount: 0.111,
            },
        ],
        vec![],
        false,
    );
}

#[test]
#[ignore]
fn test_pool_polis_ray() {
    pool_actions::run_test(
        "RDM.POLIS-RAY-V4",
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
        false,
    );
}

#[test]
#[ignore]
fn test_pool_polis_ray_latest() {
    pool_actions::run_test(
        "RDM.POLIS-RAY",
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
        false,
    );
}

#[test]
#[ignore]
fn test_pool_grape_usdc() {
    pool_actions::run_test(
        "RDM.GRAPE-USDC-V4",
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
        false,
    );
}

#[test]
#[ignore]
fn test_pool_fida_ray() {
    pool_actions::run_test(
        "RDM.FIDA-RAY-V4",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.21111111,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "RAY",
                to_token: "FIDA",
                amount: -0.5,
            },
        ],
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "FIDA",
            to_token: "RAY",
            amount: 0.0,
        }],
        false,
    );
}

#[test]
#[ignore]
fn test_pool_ray_sol() {
    pool_actions::run_test(
        "RDM.RAY-SOL-V4",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "RAY",
            amount: 0.09999999,
        }],
        vec![],
        false,
    );
}

#[test]
#[ignore]
fn test_pool_ray_sol_latest() {
    pool_actions::run_test(
        "RDM.RAY-SOL",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "RAY",
            amount: 0.09999999,
        }],
        vec![],
        false,
    );
}

#[test]
#[ignore]
fn test_pool_sol_usdc() {
    pool_actions::run_test(
        "RDM.SOL-USDC-V4",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "USDC",
            amount: 0.10000001,
        }],
        vec![],
        true,
    );
}

#[test]
#[ignore]
fn test_pool_sol_usdc_latest() {
    pool_actions::run_test(
        "RDM.SOL-USDC",
        vec![utils::Swap {
            protocol: "RDM",
            from_token: "SOL",
            to_token: "USDC",
            amount: 0.10000001,
        }],
        vec![],
        true,
    );
}

#[test]
#[ignore]
fn test_pool_msol_usdc() {
    pool_actions::run_test(
        "RDM.MSOL-USDC-V4",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "MSOL",
                amount: 0.10000001,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "USDC",
                amount: 0.1111,
            },
        ],
        vec![],
        true,
    );
}

#[test]
#[ignore]
fn test_pool_ray_usdc() {
    pool_actions::run_test(
        "RDM.RAY-USDC-V4",
        vec![
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "RAY",
                amount: 0.10000001,
            },
            utils::Swap {
                protocol: "RDM",
                from_token: "SOL",
                to_token: "USDC",
                amount: 0.09999999,
            },
        ],
        vec![],
        true,
    );
}
