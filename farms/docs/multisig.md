# Multisig

Multi-signature mode is supported for all Main Router, Vault, and Fund transactions that require admin privileges, including program upgrades. To enable multisig for Main Router use:

```sh
solana-farm-ctrl set-admins [MIN_SIGNATURES] [ADMIN_KEY1] [ADMIN_KEY2]...
```

The maximum number of admin signers is six.

To enable multisig for program upgrades, Vaults, and Funds use `solana-farm-ctrl program-set-admins`, `solana-farm-ctrl vault-set-admins`, and `solana-farm-ctrl fund-set-admins`. Alternatively, you can use [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md) or build and send raw instructions.

Multisig is fully transparent, no changes are required to how clients build or submit transactions. If a transaction requires multisig, API calls will return Ok (meaning intent was recorded on-chain) but won't be executed until enough signatures have been accumulated.

If program upgrades multisig is enabled, then upgrades must be performed via Main Router using `solana-farm-ctrl program-upgrade`, e.g.:

```sh
# write program to an on-chain buffer and get the buffer address
solana program write-buffer solana_router_raydium.so
solana program set-buffer-authority --new-buffer-authority [MULTISIG_ADDRESS] [BUFFER_ADDRESS]
solana-farm-ctrl program-upgrade [PROGRAM_ID] [BUFFER_ADDRESS]
```

Commands that initialize multisig can be used again to amend the existing set of admins (it will have to be signed by admins like other multisig transactions). To reset program upgrade multisig back to a single authority, use `solana-farm-ctrl program-set-single-authority`.

To get the current list of admins you can use `get-admins`, `program-get-admins`, `vault-get-admins` and `fund-get-admins`.
