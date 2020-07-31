#!/usr/bin/env bash

set -ex

# Test program
cd "$(dirname "$0")/.."
./do.sh update
./do.sh fmt memo --all -- --check
./do.sh build memo
./do.sh clippy memo -- --deny=warnings
./do.sh doc memo
./do.sh test memo
