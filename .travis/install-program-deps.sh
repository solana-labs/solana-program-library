# |source| this file

cargo --version
rustup install nightly
rustup component add rustfmt
rustup component add clippy --toolchain nightly
docker --version
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
sudo apt-add-repository "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-10 main"
sudo apt-get update
sudo apt-get install -y clang-7 --allow-unauthenticated
sudo apt-get install -y openssl --allow-unauthenticated
sudo apt-get install -y libssl-dev --allow-unauthenticated
sudo apt-get install -y libssl1.1 --allow-unauthenticated
sudo apt-get install -y libudev-dev
clang-7 --version
nvm install node
npm install -g typescript
node --version
