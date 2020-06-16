#!/usr/bin/env bash

cd "$(dirname "$0")/.."

set -e

./do.sh update
./do.sh build memo
./do.sh doc memo
./do.sh test memo
