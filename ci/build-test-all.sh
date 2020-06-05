#!/usr/bin/env bash

cd "$(dirname "$0")/.."

set -e

./do.sh build
./do.sh test
