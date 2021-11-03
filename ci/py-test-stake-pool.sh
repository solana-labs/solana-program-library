#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")/.."
source ./ci/solana-version.sh install

cd stake-pool/py
python3 -m venv venv
source ./venv/bin/activate
pip3 install -r requirements.txt
check_dirs=(
  "actions"
  "stake"
  "stake_pool"
  "tests"
)
flake8 "${check_dirs[@]}"
mypy "${check_dirs[@]}"
python3 -m pytest
