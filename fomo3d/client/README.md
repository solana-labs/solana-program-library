# Fomo3D JS Client

To be able to use the JS client follow these steps:

1. [install](https://docs.solana.com/cli/install-solana-cli-tools) the `solana` sdk
2. generate a master keypair
```shell
solana-keygen new
```
3. set the cluster to localnet
```shell
solana config set --url localhost
```
4. start a local validator node
```shell
solana-test-validator --reset 
```
5. build & deploy the program
```
cargo build-bpf
solana program deploy path/to/fomo3d.so
```
6. Update the program id in the `main.ts` file (line 29)
7. fund the accounts that are used for testing (alternative would be to use `airdrop` but it sometimes breaks on localnet)
```shell
# fund game creator's account
solana transfer AFe99p6byLxYfEV9E1nNumSeKdtgXm2HL5Gy5dN6icj9 10 --allow-unfunded-recipient
# fund alice's account
solana transfer Ga8HG4NzgcYkegLoJDmxJemEU1brewF2XZLNHd6B4wJ7 10 --allow-unfunded-recipient
# fund bob's account
solana transfer BxiV2mYXbBma1Kv7kxnn7cdM93oFHL4BhT9G23hiFfUP 10 --allow-unfunded-recipient 
```
8. install `node`, `npm`, `yarn`
8. run the test suite
```
cd client
yarn
yarn test
```

A couple quirks worth mentioning:
1. Version is generated randomly and allows each test run to basically start fresh
2. The reason for the numerous `describe` blocks is because unlike `it` or `test` blocks, these run synchronously, which is the desired behavior for our use-case 