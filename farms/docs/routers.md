# Main Router

Main Router is an on-chain program that handles the creation, updates, and deletion of all metadata objects: tokens, pools, farms, vaults, program IDs, and generic key-value records, such as user or vault stats.

Interaction with the Main Router can be done via [Farm Control CLI](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/farm_ctrl_cli.md), [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md), or calling instructions directly. The list of instructions can be found [here](https://github.com/solana-labs/solana-program-library/blob/master/farms/farm-sdk/src/instruction/main_router.rs).

# Protocol Routers

Protocol Routers are on-chain programs with a common interface for interaction with Raydium, Saber, and Orca Pools and Farms. They perform in and out amounts calculations and safety checks for tokens spent and received. They don't hold user funds but validate, wrap, and send instructions to the AMMs and Farms.

Interaction with Protocol Routers can be done via [Farm Client CLI](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/farm_client_cli.md), [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md), [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md), or calling instructions directly. The list of instructions can be found [here](https://github.com/solana-labs/solana-program-library/blob/master/farms/farm-sdk/src/instruction/amm.rs).
