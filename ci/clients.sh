#!/usr/bin/env bash

set -ex

cd "$(dirname "$0")/.."
cargo run --manifest-path=ci/client/Cargo.toml
