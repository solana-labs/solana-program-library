# Governance

To initialize the DAO, first build and deploy the governance program:

```sh
cd solana-program-library/governance/program
cargo build-bpf
solana program deploy --commitment finalized target/deploy/spl_governance.so
```

Then initialize the DAO using the Main Router admin account with:

```sh
solana-farm-ctrl --keypair main_admin.json governance init [DAO_PROGRAM_ADDRESS] [DAO_TOKENS_TO_MINT]
```

It will take over on-chain programs upgrade authorities (including the DAO program itself) and DAO mint. Realm authority will also be removed. DAO tokens will be deposited to the admin account for further distribution.

Farm client can be used to perform all DAO operations: create proposals, deposit tokens, sign-off, add or execute instructions, vote, etc. See help for details:

```sh
solana-farm-client governance help
```

As part of DAO initialization, SOL token custody will be created (and more tokens can be added permissionless). Custody can be used to govern all interactions with Pools, Farms, or Vaults. It is useful if a third party manages funds, and every operation must be voted on first. [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md) or [Farm Client CLI](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/farm_client_cli.md) simplify instruction creation and verification process, here is a workflow example for already initialized DAO:

```sh
solana-farm-client governance proposal-new FarmCustodyGovernance SwapTokens http://description.com 0
solana-farm-client governance signatory-add FarmCustodyGovernance 0 J7paVZ8axBfUaGFDNknc7XF3GHjVLZzvL57FaCuxjJo7
solana-farm-client governance instruction-insert-swap FarmCustodyGovernance 0 0 RDM RAY SRM 1.0 0.0
solana-farm-client -k signer.json governance sign-off FarmCustodyGovernance 0
solana-farm-client -k voter.json governance instruction-verify-swap FarmCustodyGovernance 0 0 RDM RAY SRM 1.0 0.0
solana-farm-client -k voter.json governance vote-cast FarmCustodyGovernance 0 1
solana-farm-client governance vote-finalize FarmCustodyGovernance 0
solana-farm-client -k anyone.json governance instruction-execute FarmCustodyGovernance 0 0
```
