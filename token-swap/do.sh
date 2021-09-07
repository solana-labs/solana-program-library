#!/usr/bin/env bash

set -ex
cd "$(dirname "$0")"

solana_version="1.7.11"
#export PATH="$HOME"/.local/share/solana/install/active_release/bin:"$PATH"

usage() {
    cat <<EOF
Usage: do.sh <action> <action specific arguments>
Supported actions:
    build
    build-lib
    deploy
    clean
    clippy
    doc
    dump
    new-swap
    fmt
    test
    update
EOF
}

perform_action() {
    set -ex
    case "$1" in
    build)
        (
            pushd program
            cargo build-bpf
            popd
        )
        ;;
    build-prod)
        (
            if [[ -z "${SWAP_PROGRAM_OWNER_FEE_ADDRESS}" ]]; then
                echo "Error: SWAP_PROGRAM_OWNER_FEE_ADDRESS not set"
                exit
            fi
            if [[ -z "${REQUIRED_MINT_ADDRESS}" ]]; then
                echo "Error: REQUIRED_MINT_ADDRESS not set"
                exit
            fi
            pushd program
            cargo build-bpf --features=production
            popd
        )
        ;;
    build-lib)
        (
            pushd program
            export RUSTFLAGS="${@:2}"
            cargo build
            popd
        )
        ;;
    clean)
        (
            pushd program
            cargo clean
            popd
        )
        ;;
    clippy)
        (
            cargo +nightly clippy ${@:2}
        )
        ;;
    deploy)
        (
            ./scripts/deploy-step-swap.sh $2 $3
            ./do.sh new-swap
        )
    ;;
    doc)
        (
            pushd program
            echo "generating docs ..."
            cargo doc ${@:2}
            popd
        )
        ;;
    dump)
        (
            # Dump depends on tools that are not installed by default and must be installed manually
            # - rustfilt
            pushd program
            cargo build-bpf --dump ${@:2}
            popd
        )
        ;;
    fmt)
        (
            pushd program
            cargo fmt ${@:2}
            popd
        )
        ;;
    new-swap)
            yarn --cwd js install
            yarn --cwd js start-with-test-validator
        ;;
    test)
        (
            pushd program
            cargo test-bpf ${@:2}
            popd
        )
        ;;
    update)
        (
            exit_code=0
            which solana-install || exit_code=$?
            if [ "$exit_code" -eq 1 ]; then
                echo Installing Solana tool suite ...
                sh -c "$(curl -sSfL https://release.solana.com/v${solana_version}/install)"
            fi
            solana-install update
        )
        ;;
    help)
            usage
            exit
        ;;
    *)
        echo "Error: Unknown command"
        usage
        exit
        ;;
    esac
}

if [[ $1 == "update" ]]; then
    perform_action "$1"
    exit
else
    if [[ "$#" -lt 1 ]]; then
        usage
        exit
    fi
    exit_code=0
    which solana-install || exit_code=$?
    if [ "$exit_code" -eq 1 ]; then
        ./do.sh update
    fi
fi

perform_action "$1" "${@:2}"
