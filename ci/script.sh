#!/usr/bin/env bash

set -e

cd "$(dirname "$0")/.."

_() {
  declare fold_name=$1
  shift
  echo "travis_fold:start:$fold_name"
  "$@" || exit 1
  echo "travis_fold:end:$fold_name"
}

export RUSTFLAGS="-D warnings"
export RUSTBACKTRACE=1

_ "cargo_fmt" cargo fmt --all -- --check
_ "cargo_clippy non-program" cargo +nightly clippy --workspace --all-targets -- --deny=warnings

# Run clippy again for all program crates, with the `program` feature enabled
for Xargo_toml in $(git ls-files -- '*/Xargo.toml'); do
  program_dir=$(dirname "$Xargo_toml")
  (

    cd $program_dir
    _ "clippy_program_$program_dir" cargo +nightly clippy --features=program -- --deny=warnings
  )
done

_ "ci_client" cargo run --manifest-path=ci/client/Cargo.toml

# Memo program:
_ "memo_build" ./do.sh build memo
_ "memo_test" ./do.sh test memo


# Token / Token Swap programs:
(
  export SPL_CBINDGEN=1     # <-- Force cbindgen header generation
  _ "build_lib" cargo build
)

# Check generated C headers
_ "diff_token.h" git diff --exit-code token/program/inc/token.h
_ "cc_token.sh" cc token/program/inc/token.h -o target/token.gch

_ "diff_token-swap.h" git diff --exit-code token-swap/program/inc/token-swap.h
_ "cc_token-swap.sh" cc token-swap/program/inc/token-swap.h -o target/token-swap.gch


_ "build_token" ./do.sh build token
_ "build_token-swap" ./do.sh build token-swap

_ "test_token" ./do.sh test token
# TODO: Uncomment once "Undefined symbols for architecture x86_64: _sol_create_program_address" is resolved
# _ "test token-swap" ./do.sh test token-swap


# Test token js bindings
echo "travis_fold:start:js_token"
(
  set -x
  cd token/js
  npm install
  npm run lint
  npm run flow
  tsc module.d.ts
  npm run cluster:localnet
  npm run localnet:down
  npm run localnet:update
  npm run localnet:up
  npm run start
  npm run localnet:down
)
echo "travis_fold:end:js_token"

# Test token-swap js bindings
echo "travis_fold:start:js_token-swap"
(
  set -x
  cd token-swap/js
  npm install
  npm run lint
  npm run flow

  # TODO: Uncomment once https://github.com/solana-labs/solana/issues/11465 is resolved
  # npm run cluster:localnet
  # npm run localnet:down
  # npm run localnet:update
  # npm run localnet:up
  # npm run start
  # npm run localnet:down
)
echo "travis_fold:end:js_token-swap"

exit 0
