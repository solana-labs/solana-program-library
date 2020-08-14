#!/usr/bin/env bash

set -e

cd "$(dirname "$0")/.."

_() {
  echo "travis_fold:start:_"
  SECONDS=
  (
    set -x
    "$@"
  ) || exit 1
  echo "travis_fold:end:_"
  declare elapsed_seconds=$SECONDS

  # TODO: Use "travis_time" annotations instead of this fold hack:
  echo "travis_fold:start:${elapsed_seconds}s"
  echo "travis_fold:end:${elapsed_seconds}s"
}

export RUSTFLAGS="-D warnings"
export RUSTBACKTRACE=1

_ cargo fmt --all -- --check
_ cargo +nightly clippy --workspace --all-targets -- --deny=warnings


# Build client libraries
(
  export SPL_CBINDGEN=1     # <-- Force cbindgen header generation
  _ cargo build
)
_ cargo run --manifest-path=ci/client/Cargo.toml

# Check generated C headers
_ git diff --exit-code token/program/inc/token.h
_ cc token/program/inc/token.h -o target/token.gch

_ git diff --exit-code token-swap/program/inc/token-swap.h
_ cc token-swap/program/inc/token-swap.h -o target/token-swap.gch


# Run clippy for all program crates, with the `program` feature enabled
for Xargo_toml in $(git ls-files -- '*/Xargo.toml'); do
  program_dir=$(dirname "$Xargo_toml")
  (

    cd $program_dir
    _ cargo +nightly clippy --features=program -- --deny=warnings
  )

  _ ./do.sh build "$program_dir"

  if [[ $program_dir =~ token-swap/* ]]; then
    # TODO: Remove once "Undefined symbols for architecture x86_64: _sol_create_program_address" is resolved
    _ echo "SKIPPED token-swap test due to: Undefined symbols for architecture x86_64: _sol_create_program_address"
  else
    _ ./do.sh test "$program_dir"
  fi
done


# Test token js bindings
js_token() {
  (
    set -x
    cd token/js
    time npm install
    time npm run lint
    time npm run flow
    tsc module.d.ts

    npm run cluster:localnet
    npm run localnet:down
    npm run localnet:update
    npm run localnet:up
    time npm run start
    npm run localnet:down
  )
}
_ js_token

# Test token-swap js bindings
js_token_swap() {
  (
    set -x
    cd token-swap/js
    time npm install
    time npm run lint
    time npm run flow

    # TODO: Uncomment once https://github.com/solana-labs/solana/issues/11465 is resolved
    # npm run cluster:localnet
    # npm run localnet:down
    # npm run localnet:update
    # npm run localnet:up
    # npm run start
    # npm run localnet:down
  )
}
_ js_token_swap

exit 0
