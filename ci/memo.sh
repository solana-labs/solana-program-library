#!/usr/bin/env bash

cd "$(dirname "$0")/.."

set -e

./do.sh build memo
./do.sh test memo
