# Brave THEMIS

An implementation of Brave's THEMIS research project. This project contains
two privacy-oriented smart contracts, the Policy Smart Contract (PSC) and
the Fund Smart Contract (FSC). Together, the two contracts allow users to
be compensated for engaging with ad publishers. The users do not expose
their identities or preferences.

## Build and Run the TPS demo client

The demo client simulates 1,000 users interacting with the Ristretto
version of the THEMIS on-chain program.

### Install prerequisites

Create an Ubuntu 20.04 instance with at least 8GB of memory and 20 GB of
disk space.

Install system dependencies:

```bash
sudo apt update
sudo apt install -y g++ libclang-dev pkg-config make libssl-dev libudev-dev
```

Install the Rust compiler:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Install the Solana command-line tools:

```bash
curl -sSf https://raw.githubusercontent.com/solana-labs/solana/v1.4.2/install/solana-install-init.sh | sh -s - v1.4.2
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### Build the TPS example app

Clone this git repo:

```bash
git clone https://github.com/solana-labs/solana-program-library.git
cd solana-program-library
```

Build the demo client:

```bash
cd themis/client_ristretto
cargo build --example tps
```

### Configure the default Solana wallet

Point to the testnet cluster (default is mainnet-beta):

```bash
solana config set --url http://api.testnet.solana.com
```

Create a keypair and airdrop it some SOL:

```bash
solana-keygen new --no-passphrase
solana airdrop 10
```

### Run the TPS example app

```bash
cargo run --example tps
```

You should see something like:

```bash
Seeding feepayer accounts...
Starting benchmark...
Benchmark complete.
4000 transactions in 27.880907273s (143.4673542303847 TPS)
```
