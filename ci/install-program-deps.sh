#!/usr/bin/env bash

set -e

cargo --version
cargo install rustfilt || true
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
sudo apt-add-repository "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main"
sudo apt-get update
sudo apt-get install -y clang-7 --allow-unauthenticated
sudo apt-get install -y openssl --allow-unauthenticated
sudo apt-get install -y libssl-dev --allow-unauthenticated
sudo apt-get install -y libssl1.1 --allow-unauthenticated
sudo apt-get install -y libudev-dev
clang-7 --version

if [[ -n $SOLANA_VERSION ]]; then
  sh -c "$(curl -sSfL https://release.solana.com/$SOLANA_VERSION/install)"
fi
export PATH=/home/runner/.local/share/solana/install/active_release/bin:"$PATH"

solana --version
cargo build-bpf --version
