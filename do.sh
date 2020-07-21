#!/usr/bin/env bash

cd "$(dirname "$0")"

usage() {
    cat <<EOF

Usage: do.sh <action> <project> <action specific arguments>

Supported actions:
    build
    clean
    clippy
    doc
    dump
    fmt
    test
    update

Supported projects:
    all
    any directory containing a Cargo.toml file

EOF
}

sdkParentDir=bin
sdkDir="$sdkParentDir"/bpf-sdk
targetDir="$PWD"/"$2"/target
profile=bpfel-unknown-unknown/release

perform_action() {
    set -e
    case "$1" in
    build)
        "$sdkDir"/rust/build.sh "$2"

        so_path="$targetDir/$profile"
        so_name="spl_${3%/}"
        if [ -f "$so_path/${so_name}.so" ]; then
            cp "$so_path/${so_name}.so" "$so_path/${so_name}_debug.so"
            "$sdkDir"/dependencies/llvm-native/bin/llvm-objcopy --strip-all "$so_path/${so_name}.so" "$so_path/$so_name.so"
        fi
        ;;
    build-native)
        (
            cd "$2"
            echo "build $2"
            export RUSTFLAGS="${@:4}"
            cargo build
        )
        ;;
    clean)
        "$sdkDir"/rust/clean.sh "$2"
        ;;
    test)
        (
            cd "$2"
            echo "test $2"
            cargo +nightly test ${@:4}
        )
        ;;
    clippy)
        (
            cd "$2"
            echo "clippy $2"
            cargo +nightly clippy ${@:4}
        )
        ;;
    fmt)
        (
            cd "$2"
            echo "formatting $2"
            cargo fmt ${@:4}
        )
        ;;
    doc)
        (
            cd "$2"
            echo "generating docs $2"
            cargo doc ${@:4}
        )
        ;;
    update)
        mkdir -p $sdkParentDir
        ./bpf-sdk-install.sh $sdkParentDir
        ./do.sh clean all
        ;;
    dump)
        # Dump depends on tools that are not installed by default and must be installed manually
        # - greadelf
        # - rustfilt
        (
            download_bpf_sdk
            pwd
            "$0" build "$3"

            cd "$3"
            so_path="$targetDir/$profile"
            so_name="solana_bpf_${3%/}"
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
                    <"${dump}-mangled.txt" |
                    rustfilt \
                        >"${dump}.txt"
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
if [[ $1 == "update" ]]; then
    perform_action "$1"
    exit
else
    if [[ "$#" -lt 2 ]]; then
        usage
        exit
    fi
    if [[ ! -d "$sdkDir" ]]; then
        ./do.sh update
    fi
fi

if [[ $2 == "all" ]]; then
    # Perform operation on all projects
    for project in */; do
        if [[ -f "$project"Cargo.toml ]]; then
            perform_action "$1" "$PWD/$project" "$project" ${@:3}
        else
            continue
        fi
    done
else
    # Perform operation on requested project
    perform_action "$1" "$PWD/$2" "$2" "${@:3}"
fi
