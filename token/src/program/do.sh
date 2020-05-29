#!/usr/bin/env bash

cd "$(dirname "$0")"

usage() {
    cat <<EOF

Usage: do.sh action

Supported actions:
    build
    clean
    test
    clippy
    fmt

EOF
}

sdkDir=../../node_modules/@solana/web3.js/bpf-sdk
targetDir="$PWD"/target
profile=bpfel-unknown-unknown/release

perform_action() {
    set -e
    case "$1" in
    build)
        "$sdkDir"/rust/build.sh "$PWD"
        
        so_path="$targetDir/$profile"
        so_name="spl_token"
        if [ -f "$so_path/${so_name}.so" ]; then
            cp "$so_path/${so_name}.so" "$so_path/${so_name}_debug.so"
            "$sdkDir"/dependencies/llvm-native/bin/llvm-objcopy --strip-all "$so_path/${so_name}.so" "$so_path/$so_name.so"
        fi
        ;;
    clean)
        "$sdkDir"/rust/clean.sh "$PWD"
        ;;
    test)
        echo "test"
        shift
        cargo +nightly test $@
        ;;
    clippy)
        echo "clippy"
        cargo +nightly clippy
        ;;
    fmt)
        echo "formatting"
        cargo fmt
        ;;
    dump)
        # Dump depends on tools that are not installed by default and must be installed manually
        # - greadelf
        # - rustfilt
        (
            pwd
            "$0" build

            so_path="$targetDir/$profile"
            so_name="solana_bpf_token"
            so="$so_path/${so_name}_debug.so"
            dump="$so_path/${so_name}-dump"

            if [ -f "$so" ]; then
                ls \
                    -la \
                    "$so" \
                    >"${dump}-mangled.txt"
                greadelf \
                    -aW \
                    "$so" \
                    >>"${dump}-mangled.txt"
                "$sdkDir/dependencies/llvm-native/bin/llvm-objdump" \
                    -print-imm-hex \
                    --source \
                    --disassemble \
                    "$so" \
                    >>"${dump}-mangled.txt"
                sed \
                    s/://g \
                    < "${dump}-mangled.txt" \
                    | rustfilt \
                    > "${dump}.txt"
            else
                echo "Warning: No dump created, cannot find: $so"
            fi
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

set -e

perform_action "$@"
