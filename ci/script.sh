#!/usr/bin/env bash

set -e

cd "$(dirname "$0")/.."

travis_cmd_prelude() {
  if [[ -n "$TRAVIS" ]]; then
    echo "travis_fold:start:_"
  fi
}

travis_cmd_postlude() {
  declare elapsed_seconds=$1
  if [[ -n "$TRAVIS" ]]; then
    echo "travis_fold:end:_"
    # TODO: Use "travis_time" annotations instead of this fold hack:
    echo "travis_fold:start:${elapsed_seconds}s"
    echo "travis_fold:end:${elapsed_seconds}s"
  fi
}

_() {
  travis_cmd_prelude
  SECONDS=
  (
    set -x
    "$@"
  ) || exit 1
  travis_cmd_postlude $SECONDS
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

# For all BPF programs
for Xargo_toml in $(git ls-files -- '*/Xargo.toml'); do
  program_dir=$(dirname "$Xargo_toml")
  (
    # Run clippy for all program crates, with the `program` feature enabled
    cd $program_dir
    _ cargo +nightly clippy --features=program -- --deny=warnings
  )

  _ ./do.sh build "$program_dir"

  _ ./do.sh test "$program_dir"

  _ ./do.sh dump "$program_dir"
done

# Run SPL Token's performance monitor
_ cargo test --manifest-path=token/perf-monitor/Cargo.toml -- --nocapture
_ cargo test --manifest-path=themis/client_bn/Cargo.toml -- --nocapture
_ cargo test --manifest-path=themis/client_ristretto/Cargo.toml -- --nocapture


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
  time npm run flow || exit $?
  tsc module.d.ts || exit $?

  npm run cluster:localnet || exit $?
  npm run localnet:down
  npm run localnet:update || exit $?
  npm run localnet:up || exit $?
  time npm run start || exit $?
  npm run localnet:down
}
_ js_token_swap

# Test token-lending js bindings
js_token_lending() {
  cd token-lending/js
  time npm install || exit $?
  time npm run lint || exit $?
  time npm run build || exit $?

  npm run cluster:localnet || exit $?
  npm run localnet:down
  npm run localnet:update || exit $?
  npm run localnet:up || exit $?
  time npm run start || exit $?
  npm run localnet:down
}
_ js_token_lending

exit 0
