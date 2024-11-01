## SPL Token 2022 Program Tests

All of the end-to-end tests for spl-token-2022 exist in this directory.

### Requirements

These tests require other built on-chain programs, including:

* spl-instruction-padding
* spl-transfer-hook-example
* spl-transfer-hook-example-downgrade
* spl-transfer-hook-example-fail
* spl-transfer-hook-example-success
* spl-transfer-hook-example-swap
* spl-transfer-hook-example-swap-with-fee

Built versions of these programs exist in `tests/fixtures`, and may be
regenerated from the following places in this repo:

* instruction-padding/program
* token/transfer-hook/example
* token/program-2022-test/transfer-hook-test-programs/downgrade
* token/program-2022-test/transfer-hook-test-programs/fail
* token/program-2022-test/transfer-hook-test-programs/success
* token/program-2022-test/transfer-hook-test-programs/swap
* token/program-2022-test/transfer-hook-test-programs/swap-with-fee
