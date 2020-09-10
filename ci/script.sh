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
_ cargo build
_ cargo run --manifest-path=utils/test-client/Cargo.toml


#  # Check generated C headers
#  _ cargo run --manifest-path=utils/cgen/Cargo.toml
#
#  _ git diff --exit-code token/program/inc/token.h
#  _ cc token/program/inc/token.h -o target/token.gch
#  _ git diff --exit-code token-swap/program/inc/token-swap.h
#  _ cc token-swap/program/inc/token-swap.h -o target/token-swap.gch


# Run clippy for all program crates, with the `program` feature enabled
for Xargo_toml in $(git ls-files -- '*/Xargo.toml'); do
  program_dir=$(dirname "$Xargo_toml")
  (

    cd $program_dir
    _ cargo +nightly clippy --features=program -- --deny=warnings
  )

  _ ./do.sh build "$program_dir"

  _ ./do.sh test "$program_dir"
done

# Run SPL Token's performance monitor
cargo test --manifest-path=token/perf-monitor/Cargo.toml -- --nocapture


# Test token js bindings
js_token() {
  cd token/js
  time npm install || exit $?
  time npm run lint || exit $?
  time npm run flow || exit $?
  tsc module.d.ts || exit $?

  npm run cluster:localnet || exit $?
  npm run localnet:down
  npm run localnet:update || exit $?
  npm run localnet:up || exit $?
  time npm run start || exit $?
  time PROGRAM_VERSION=2.0.4 npm run start || exit $?
  npm run localnet:down
}
_ js_token

# Test token-swap js bindings
js_token_swap() {
  cd token-swap/js
  time npm install || exit $?
  time npm run lint || exit $?

  # TODO: Restore flow
  # time npm run flow || exit $?

  # TODO re-enable after investigating CI issues
  # https://github.com/solana-labs/solana-program-library/pull/408
  # npm run cluster:localnet || exit $?
  # npm run localnet:down
  # npm run localnet:update || exit $?
  # npm run localnet:up || exit $?
  # npm run start || exit $?
  # npm run localnet:down
}
_ js_token_swap

exit 0
