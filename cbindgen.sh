#!/usr/bin/env bash

cd "$(dirname "$0")"
set -x

# Cargo.lock can cause older spl-token bindings to be generated?  Move it out of
# the way...
mv -f Cargo.lock Cargo.lock.org

cargo run --manifest-path=utils/cgen/Cargo.toml
exitcode=$?

mv -f Cargo.lock.org Cargo.lock

exit $exitcode
