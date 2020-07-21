#!/usr/bin/env bash

cd "$(dirname "$0")/.."

set -e

./do.sh update
./do.sh fmt memo --all -- --check
./do.sh build-native memo -D warnings
./do.sh build memo
./do.sh clippy memo -- --deny=warnings
./do.sh doc memo
./do.sh test memo
