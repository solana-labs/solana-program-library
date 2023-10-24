## Transfer-Hook Example

Full example program and tests implementing the `spl-transfer-hook-interface`,
to be used for testing a program that calls into the `spl-transfer-hook-interface`.

See the
[SPL Transfer Hook Interface](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook/interface)
code for more information.

### Example usage of example

When testing your program that uses `spl-transfer-hook-interface`, you can also
import this crate, and then use it with `solana-program-test`, ie:

```rust
use {
    solana_program_test::{processor, ProgramTest},
    solana_sdk::{account::Account, instruction::AccountMeta},
    spl_transfer_hook_example::state::example_data,
    spl_transfer_hook_interface::get_extra_account_metas_address,
};

#[test]
fn my_program_test() {
    let mut program_test = ProgramTest::new(
        "my_program",
        my_program_id,
        processor!(my_program_processor),
    );

    let transfer_hook_program_id = Pubkey::new_unique();
    program_test.prefer_bpf(false); // BPF won't work, unless you've built this from scratch!
    program_test.add_program(
        "spl_transfer_hook_example",
        transfer_hook_program_id,
        processor!(spl_transfer_hook_example::processor::process),
    );

    let mint = Pubkey::new_unique();
    let extra_accounts_address = get_extra_account_metas_address(&mint, &transfer_hook_program_id);
    let account_metas = vec![
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        },
    ];
    let data = example_data(&account_metas);
    program_test.add_account(
        extra_accounts_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data,
            owner: transfer_hook_program_id,
            ..Account::default()
        },
    );

    // run your test logic!
}
```
