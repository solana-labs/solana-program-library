#!/usr/bin/env bash

set -ex

wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
sudo apt-add-repository "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main"
sudo apt-get update
sudo apt-get install -y clang-7 --allow-unauthenticated
sudo apt-get install -y openssl --allow-unauthenticated
sudo apt-get install -y libssl-dev --allow-unauthenticated
sudo apt-get install -y libssl1.1 --allow-unauthenticated
sudo apt-get install -y libudev-dev
sudo apt-get install -y binutils-dev
sudo apt-get install -y libunwind-dev
clang-7 --version
